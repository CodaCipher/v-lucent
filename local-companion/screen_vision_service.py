"""
FastAPI service for screen capture + Moondream2 summary.
Defaults: width=960px, interval=2s, WebP quality=75, port=8777.
Auto-starts capture loop on startup; no masking.
"""
import os
import time
import threading
import base64
import json
from io import BytesIO
from typing import Optional

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
from PIL import Image
import requests

try:
    import mss  # type: ignore
except ImportError as e:  # pragma: no cover
    raise RuntimeError("mss is required for screen capture") from e

try:
    import uvicorn  # type: ignore
except ImportError as e:  # pragma: no cover
    raise RuntimeError("uvicorn is required to run the service") from e

# Optional: dxcam fallback if mss fails on some games/DRM
try:  # pragma: no cover - optional
    import dxcam  # type: ignore
except Exception:  # pragma: no cover
    dxcam = None

# Vision provider config (default: OpenRouter vision-capable model)
VISION_PROVIDER = os.getenv("VISION_API_TYPE", "openrouter").strip().lower()
VISION_MODEL = os.getenv("VISION_MODEL", "meta-llama/llama-3.2-11b-vision-instruct")
VISION_API_KEY = os.getenv("VISION_API_KEY", "")
VISION_OLLAMA_ENDPOINT = os.getenv("VISION_OLLAMA_ENDPOINT", os.getenv("OLLAMA_ENDPOINT", "http://127.0.0.1:11434"))
VISION_OPENCLAW_ENDPOINT = os.getenv("VISION_OPENCLAW_ENDPOINT", os.getenv("OPENCLAW_ENDPOINT", "http://127.0.0.1:18789"))

PROMPT = (
    "You see a screenshot. Summarize key UI/game HUD, visible text, current state, "
    "and likely goal. Keep under 120 words."
)

PORT = int(os.getenv("VISION_PORT", "8777"))
DEFAULT_INTERVAL = float(os.getenv("VISION_INTERVAL", "2.0"))
DEFAULT_WIDTH = int(os.getenv("VISION_WIDTH", "960"))
DEFAULT_FORMAT = os.getenv("VISION_FORMAT", "webp")  # webp|png
DEFAULT_QUALITY = int(os.getenv("VISION_QUALITY", "75"))

app = FastAPI(title="Screen Vision Service", version="0.1.0")

_state_lock = threading.Lock()
_running_event = threading.Event()
_latest_summary: dict[str, Optional[str]] = {"ts": None, "summary": None}
_config = {
    "interval": DEFAULT_INTERVAL,
    "width": DEFAULT_WIDTH,
    "format": DEFAULT_FORMAT,
    "quality": DEFAULT_QUALITY,
}


class ConfigRequest(BaseModel):
    interval: Optional[float] = None  # seconds
    width: Optional[int] = None  # resize width
    format: Optional[str] = None  # webp/png
    quality: Optional[int] = None  # webp quality


def _api_completion(image_b64: str, prompt: str) -> str:
    provider = VISION_PROVIDER
    url = None
    if provider == "openrouter":
        url = "https://openrouter.ai/api/v1/chat/completions"
    elif provider == "openai":
        url = "https://api.openai.com/v1/chat/completions"
    elif provider == "groq":
        url = "https://api.groq.com/openai/v1/chat/completions"
    elif provider == "openclaw":
        url = VISION_OPENCLAW_ENDPOINT
    elif provider == "ollama":
        url = VISION_OLLAMA_ENDPOINT.rstrip("/") + "/api/chat"
    else:
        return "Vision provider not supported"

    headers = {"Content-Type": "application/json"}
    if provider in {"openrouter", "openai", "groq", "openclaw"}:
        if not VISION_API_KEY and provider != "openclaw":
            return "Vision API key missing"
        if VISION_API_KEY:
            headers["Authorization"] = f"Bearer {VISION_API_KEY}"
        payload = {
            "model": VISION_MODEL,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": prompt},
                        {
                            "type": "image_url",
                            "image_url": {"url": f"data:image/png;base64,{image_b64}"},
                        },
                    ],
                }
            ],
        }
    elif provider == "ollama":
        payload = {
            "model": VISION_MODEL,
            "messages": [
                {
                    "role": "user",
                    "content": prompt,
                    "images": [image_b64],
                }
            ],
        }
    else:
        return "Vision provider not supported"
    try:
        resp = requests.post(url, headers=headers, json=payload, timeout=40)
    except Exception as e:  # pragma: no cover
        print(f"[vision] request failed: {e}")
        return "Vision backend unreachable"
    if not resp.ok:
        print(f"[vision] HTTP {resp.status_code}: {resp.text}")
        return f"Vision backend error: {resp.status_code}"
    try:
        data = resp.json()
    except Exception as e:  # pragma: no cover
        print(f"[vision] parse error: {e}")
        return "Vision backend parse error"
    choice = (
        data.get("choices", [{}])[0]
        .get("message", {})
        .get("content")
    )
    if isinstance(choice, str):
        return choice.strip()
    return "Vision backend returned no text"


def _dispatch_completion(image_b64: str, prompt: str) -> str:
    return _api_completion(image_b64, prompt)


def _capture_with_mss(width: int) -> Optional[Image.Image]:
    with mss.mss() as sct:
        monitor = sct.monitors[1]
        raw = sct.grab(monitor)
        img = Image.frombytes("RGB", raw.size, raw.rgb)
    if img.width == 0 or img.height == 0:
        return None
    if img.width != width:
        h = int(img.height * (width / img.width))
        img = img.resize((width, h))
    return img


def _capture_with_dxcam(width: int) -> Optional[Image.Image]:  # pragma: no cover - optional
    if dxcam is None:
        return None
    camera = dxcam.create(output_color="RGB")
    frame = camera.grab()
    if frame is None:
        return None
    img = Image.fromarray(frame)
    if img.width != width:
        h = int(img.height * (width / img.width))
        img = img.resize((width, h))
    return img


def capture_frame(width: int) -> Optional[Image.Image]:
    try:
        return _capture_with_mss(width)
    except Exception as e:
        print(f"[vision] mss capture failed: {e}")
    # fallback to dxcam if available
    try:
        return _capture_with_dxcam(width)
    except Exception as e:  # pragma: no cover
        print(f"[vision] dxcam capture failed: {e}")
    return None


def describe_image(img: Image.Image) -> str:
    buf = BytesIO()
    img.save(buf, format="PNG")
    b64 = base64.b64encode(buf.getvalue()).decode("utf-8")
    return _dispatch_completion(b64, PROMPT)


def encode_image(img: Image.Image, fmt: str, quality: int) -> bytes:
    buf = BytesIO()
    save_kwargs = {}
    if fmt.lower() == "webp":
        save_kwargs = {"format": "WEBP", "quality": quality}
    elif fmt.lower() == "png":
        save_kwargs = {"format": "PNG"}
    else:
        save_kwargs = {"format": "WEBP", "quality": quality}
    img.save(buf, **save_kwargs)
    return buf.getvalue()


def _loop():
    while _running_event.is_set():
        cfg = _config.copy()
        if cfg["interval"] <= 0:
            time.sleep(0.25)
            continue
        img = capture_frame(cfg["width"])
        if img is None:
            time.sleep(cfg["interval"])
            continue
        summary = describe_image(img)
        with _state_lock:
            _latest_summary["ts"] = time.time()
            _latest_summary["summary"] = summary
        time.sleep(cfg["interval"])


@app.post("/once")
def capture_once():
    cfg = _config.copy()
    img = capture_frame(cfg["width"])
    if img is None:
        raise HTTPException(status_code=500, detail="Capture failed")
    summary = describe_image(img)
    with _state_lock:
        _latest_summary["ts"] = time.time()
        _latest_summary["summary"] = summary
    return _latest_summary.copy()
            continue
        summary = describe_image(img)
        with _state_lock:
            _latest_summary["ts"] = time.time()
            _latest_summary["summary"] = summary
        time.sleep(cfg["interval"])


@app.on_event("startup")
def _on_startup():
    print("[vision] service starting; loop auto-start")
    _running_event.set()
    t = threading.Thread(target=_loop, daemon=True)
    t.start()


@app.on_event("shutdown")
def _on_shutdown():
    print("[vision] service shutting down")
    _running_event.clear()


@app.get("/health")
def health():
    return {
        "status": "ok",
        "interval": _config["interval"],
        "width": _config["width"],
        "format": _config["format"],
        "quality": _config["quality"],
        "provider": VISION_PROVIDER,
        "model": VISION_MODEL,
    }


@app.get("/latest")
def latest():
    with _state_lock:
        return _latest_summary.copy()


@app.post("/config")
def update_config(cfg: ConfigRequest):
    if cfg.interval is not None and cfg.interval > 0:
        _config["interval"] = cfg.interval
    if cfg.width is not None and cfg.width > 0:
        _config["width"] = cfg.width
    if cfg.format:
        _config["format"] = cfg.format.lower()
    if cfg.quality is not None and 1 <= cfg.quality <= 100:
        _config["quality"] = cfg.quality
    return _config


def main():  # pragma: no cover
    import argparse

    global PORT

    parser = argparse.ArgumentParser(description="Screen Vision Service")
    parser.add_argument("--host", default=os.getenv("VISION_HOST", "127.0.0.1"))
    parser.add_argument("--port", type=int, default=PORT)
    parser.add_argument("--interval", type=float, default=DEFAULT_INTERVAL)
    parser.add_argument("--width", type=int, default=DEFAULT_WIDTH)
    parser.add_argument("--format", default=DEFAULT_FORMAT, choices=["webp", "png"])
    parser.add_argument("--quality", type=int, default=DEFAULT_QUALITY)
    args = parser.parse_args()

    # apply runtime config
    PORT = args.port
    _config["interval"] = max(0.1, args.interval)
    _config["width"] = max(1, args.width)
    _config["format"] = args.format
    _config["quality"] = max(1, min(100, args.quality))

    uvicorn.run(app, host=args.host, port=PORT, log_level="info")


if __name__ == "__main__":  # pragma: no cover
    main()

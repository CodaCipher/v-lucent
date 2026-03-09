import argparse
import io
import os
import subprocess
from typing import Tuple

from flask import Flask, jsonify, request
import soundfile as sf

try:
    import torch
except Exception:
    torch = None

try:
    from faster_whisper import WhisperModel
except ImportError as exc:
    raise SystemExit("[STT] faster-whisper is required. pip install faster-whisper") from exc

app = Flask(__name__)


def pick_device_compute() -> Tuple[str, str]:
    if torch is not None and torch.cuda.is_available():
        return "cuda", "float16"
    return "cpu", "int8_float16"


def load_model() -> WhisperModel:
    device, compute_type = pick_device_compute()
    model_name = os.getenv("STT_MODEL", "distil-large-v3")
    print(f"[STT] Loading model={model_name} device={device} compute_type={compute_type}")
    model = WhisperModel(
        model_name,
        device=device,
        compute_type=compute_type,
        cpu_threads=int(os.getenv("STT_CPU_THREADS", "4")),
    )
    return model


MODEL = load_model()


@app.route("/health", methods=["GET"])
def health():
    return jsonify({"status": "ok"})


def decode_to_wav_pcm16(audio_bytes: bytes) -> Tuple[bytes, int]:
    """Decode arbitrary audio (webm/opus) to wav pcm16 mono 16k using ffmpeg."""
    cmd = [
        "ffmpeg",
        "-hide_banner",
        "-loglevel",
        "error",
        "-i",
        "pipe:0",
        "-vn",
        "-acodec",
        "pcm_s16le",
        "-ac",
        "1",
        "-ar",
        "16000",
        "-f",
        "wav",
        "pipe:1",
    ]
    proc = subprocess.run(cmd, input=audio_bytes, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.decode("utf-8", errors="replace"))
    return proc.stdout, 16000


@app.route("/stt", methods=["POST"])
def stt():
    if "file" not in request.files:
        return jsonify({"error": "missing file"}), 400

    file = request.files["file"]
    audio_bytes = file.read()
    if not audio_bytes:
        return jsonify({"error": "empty file"}), 400

    try:
        wav_bytes, sr = decode_to_wav_pcm16(audio_bytes)
    except Exception as e:
        return jsonify({"error": f"ffmpeg decode failed: {e}"}), 500

    try:
        audio_np, sr_read = sf.read(io.BytesIO(wav_bytes), dtype="float32")
    except Exception as e:
        return jsonify({"error": f"wav parse failed: {e}"}), 500

    if audio_np.ndim == 2:
        audio_np = audio_np.mean(axis=1)

    try:
        segments, info = MODEL.transcribe(
            audio_np,
            beam_size=5,
            vad_filter=True,
            language=None,
        )
        text_parts = [seg.text for seg in segments]
        text = " ".join(text_parts).strip()
        return jsonify({
            "text": text,
            "language": info.language,
        })
    except Exception as e:
        return jsonify({"error": f"transcribe failed: {e}"}), 500


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, default=5001)
    args = parser.parse_args()
    app.run(host="127.0.0.1", port=args.port)


if __name__ == "__main__":
    main()

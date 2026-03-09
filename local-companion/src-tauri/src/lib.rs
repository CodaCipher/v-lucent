use std::process::Command;
use std::os::windows::process::CommandExt;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use screenshots::Screen;
use screenshots::image::{DynamicImage, ImageOutputFormat};
use std::io::Cursor;
use tokio::time::timeout;
use futures::stream::StreamExt;
use futures::SinkExt;
use reqwest::Client as HttpClient;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::sync::Mutex as StdMutex;
use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio_tungstenite::tungstenite::Message;
use axum::{
    routing::{post, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use once_cell::sync::OnceCell;
use ort::{init, value::Tensor};
use ort::session::Session;
use tokenizers::Tokenizer;
const VISION_SCRIPT_CONTENT: &str = include_str!("../../screen_vision_service.py");
const VISION_REQS_CONTENT: &str = include_str!("../../vision_requirements.txt");
use ndarray::{Array1, Array2, Array3, ArrayView2, Axis};
use std::path::Path;
use std::path::PathBuf;
use std::env;
use std::fs;
use tower_http::cors::CorsLayer;
use tauri_plugin_fs;
use std::time::{Duration, Instant};

type ClientMap = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;

// === Local emotion classifier (MiniLM ONNX + prototype cosine) ===
static EMO_SESSION: OnceCell<StdMutex<Session>> = OnceCell::new();
static EMO_TOKENIZER: OnceCell<Tokenizer> = OnceCell::new();
static EMO_PROTOS: OnceCell<HashMap<String, Array1<f32>>> = OnceCell::new();

const EMOTION_LABELS: [&str; 11] = [
    "happy", "sad", "angry", "surprise", "fear", "neutral", "relaxed",
    "horny", "aroused", "dominant", "submissive",
];

fn proto_texts() -> HashMap<&'static str, Vec<&'static str>> {
    HashMap::from([
        ("happy", vec![
            "I’m so happy right now.", "This makes me smile.", "I feel joyful and light.",
            "That’s wonderful news.", "I’m delighted with this.", "I’m in a great mood.",
            "I’m thrilled.", "This is fantastic.", "I’m really pleased.", "Everything feels bright.",
        ]),
        ("sad", vec![
            "I feel really down today.", "This makes me want to cry.", "I’m heartbroken.",
            "I feel empty inside.", "I’m overwhelmed by sadness.", "I’m feeling blue.",
            "I’m disappointed.", "I’m sorrowful.", "This hurts deeply.", "I’m upset and low.",
        ]),
        ("angry", vec![
            "This makes me furious.", "I’m really annoyed right now.", "I can’t stand this anymore.",
            "I’m fuming with anger.", "This really ticks me off.", "I’m enraged.", "I’m boiling with anger.",
            "This frustrates me.", "I’m livid about this.", "I’m mad right now.",
        ]),
        ("surprise", vec![
            "Wow, I didn’t expect that.", "That’s a shocker.", "I’m completely surprised.",
            "This caught me off guard.", "I can’t believe it happened.", "That was unexpected.",
            "I’m astonished.", "I’m startled by this.", "This is surprising.", "I’m amazed right now.",
        ]),
        ("fear", vec![
            "I’m scared of this.", "This makes me nervous.", "I feel threatened.",
            "I’m afraid of what’s next.", "This is frightening me.", "I’m worried and tense.",
            "I’m anxious about this.", "This terrifies me.", "I’m uneasy right now.", "I feel danger coming.",
        ]),
        ("neutral", vec![
            "I have no strong feelings about this.", "This seems ordinary.", "I feel indifferent.",
            "Nothing stands out here.", "I’m just observing calmly.", "I’m neutral on this.",
            "It’s neither good nor bad.", "I’m fine either way.", "This is okay.", "I’m steady and calm.",
        ]),
        ("relaxed", vec![
            "I feel calm and at ease.", "This is soothing.", "I’m chilling peacefully.",
            "I’m comfortable and relaxed.", "Everything feels tranquil.", "I’m unwinding now.",
            "This is restful.", "I’m serene.", "I’m cool and easygoing.", "This is calming me.",
        ]),
        ("horny", vec![
            "I’m feeling really horny right now.", "You’re turning me on so much.", "I’m craving your touch.",
            "I feel intensely aroused.", "You make me wet just thinking about you.", "You make me hard just thinking about you.",
            "I’m burning with desire.", "I’m dripping with need.", "I’m in a lustful mood.", "I need you badly.",
        ]),
        ("aroused", vec![
            "I’m getting so aroused.", "That’s making my body react.", "I feel heat rising inside me.",
            "I’m in a sensual mood.", "This is stirring me up.", "I’m feeling flushed.", "I’m excited physically.",
            "I’m warming up with desire.", "This is turning me on gently.", "I’m starting to crave more.",
        ]),
        ("dominant", vec![
            "I own you. You will kneel to me.",
            "I command you and you obey immediately.",
            "Your only job is to follow my orders.",
            "Do exactly as I say without hesitation.",
            "I control every move you make.",
            "You exist to serve me; I am above you.",
            "Kneel and stay there until I allow otherwise.",
            "You answer only to me; I am in charge.",
            "I punish disobedience; you will comply.",
            "My authority is absolute; your obedience is expected.",
        ]),
        ("submissive", vec![
            "I obey you; your word is my command.",
            "Tell me what to do and I will do it.",
            "I exist to serve you and follow orders.",
            "I kneel for you and wait for instructions.",
            "Your control over me is total; I comply.",
            "I accept punishment and submit to your rule.",
            "I’m devoted to obeying you completely.",
            "I follow every command you give without question.",
            "I am beneath you and ready to serve.",
            "I crave to be under your control and direction.",
        ]),
    ])
}

fn clean_ollama_response(raw: &str) -> (String, String) {
    let mut candidate = strip_code_fences(raw);
    // Attempt to parse as JSON first
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&candidate) {
        if let Some(text) = value.get("text").and_then(|v| v.as_str()).map(|s| s.trim().to_string()) {
            let emotion = value
                .get("emotion")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "neutral".to_string());
            return (text, emotion);
        }
    }

    let mut emotion: Option<String> = None;
    let mut cleaned_lines: Vec<String> = Vec::new();

    for line in candidate.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_lowercase();
        if lower.contains("without the json field") {
            continue;
        }
        if let Some(found) = extract_feeling(trimmed) {
            if emotion.is_none() {
                emotion = Some(found);
            }
            continue;
        }
        cleaned_lines.push(trimmed.to_string());
    }

    if cleaned_lines.is_empty() {
        cleaned_lines.push(candidate.trim().to_string());
    }

    let text = cleaned_lines.join("\n\n");
    let emotion = emotion.unwrap_or_else(|| "neutral".to_string());
    (text, emotion)
}

fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }

    // Remove leading fence and optional language tag
    let mut content = &trimmed[3..];
    if let Some(pos) = content.find('\n') {
        content = &content[pos + 1..];
    } else {
        content = "";
    }

    if let Some(pos) = content.rfind("```") {
        content[..pos].trim().to_string()
    } else {
        content.trim().to_string()
    }
}

fn extract_feeling(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("(feeling:") && lower.ends_with(')') {
        let inner = trimmed.trim_matches(|c| c == '(' || c == ')');
        if let Some(pos) = inner.find(':') {
            let value = inner[pos + 1..].trim().trim_matches(|c| c == '.' || c == '!');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn start_stt_daemon(settings: &AppSettings) -> Result<(), String> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    // stt_server.py is located alongside the binary (resources copied at build time)
    let script = exe_dir.join("stt_server.py");
    if !script.exists() {
        // Fallback: try relative to manifest (dev mode)
        let alt = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stt_server.py");
        if alt.exists() {
            return Command::new("python")
                .arg(alt)
                .arg("--port")
                .arg(settings.stt_port.to_string())
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("Failed to start stt_server.py (alt): {}", e));
        }
        return Err("stt_server.py not found".to_string());
    }

    Command::new("python")
        .arg(script)
        .arg("--port")
        .arg(settings.stt_port.to_string())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to start stt_server.py: {}", e))
}

async fn handle_capture_and_ask(Json(body): Json<CaptureAskRequest>) -> Json<serde_json::Value> {
    let default_prompt = "You see a screenshot. Summarize key UI/game HUD, visible text, current state, and likely goal. Keep under 120 words.".to_string();
    let prompt = body.question.unwrap_or(default_prompt);

    println!("[capture] request received");

    // Capture first screen
    let screens = match Screen::all() {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({"error": format!("List screens failed: {}", e)}));
        }
    };
    println!("[capture] screens available: {}", screens.len());
    let screen = match screens.first() {
        Some(s) => s,
        None => {
            return Json(json!({"error": "No screen available"}));
        }
    };
    let image = match screen.capture() {
        Ok(img) => img,
        Err(e) => {
            return Json(json!({"error": format!("Capture failed: {}", e)}));
        }
    };
    println!("[capture] capture ok, encoding PNG");
    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);
    let dyn_img = DynamicImage::ImageRgba8(image);
    if let Err(e) = dyn_img.write_to(&mut cursor, ImageOutputFormat::Png) {
        return Json(json!({"error": format!("PNG encode failed: {}", e)}));
    }
    let b64 = B64.encode(png_bytes);
    let data_url = format!("data:image/png;base64,{}", b64);

    let settings = load_settings();
    let provider = if !settings.vision_api_type.trim().is_empty() {
        settings.vision_api_type.trim().to_string()
    } else {
        settings.api_type.trim().to_string()
    };
    let model = if !settings.vision_model.trim().is_empty() {
        settings.vision_model.trim().to_string()
    } else {
        settings.openrouter_provider.trim().to_string()
    };
    let api_key = if !settings.vision_api_key.trim().is_empty() {
        settings.vision_api_key.trim().to_string()
    } else {
        settings.openrouter_api_key.trim().to_string()
    };
    let ollama_endpoint = if !settings.ollama_endpoint.trim().is_empty() {
        settings.ollama_endpoint.trim().to_string()
    } else {
        "http://127.0.0.1:11434".to_string()
    };
    let openclaw_endpoint = if !settings.openclaw_endpoint.trim().is_empty() {
        settings.openclaw_endpoint.trim().to_string()
    } else {
        "http://127.0.0.1:18789".to_string()
    };

    let provider_lc = provider.to_lowercase();
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .connect_timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Json(json!({"error": format!("Client build failed: {}", e)})),
    };

    println!("[capture] provider={} model={}", provider_lc, model);

    if !matches!(provider_lc.as_str(), "openrouter" | "openai" | "groq" | "openclaw" | "ollama") {
        return Json(json!({"error": format!("Unsupported provider: {}", provider)}));
    }

    if provider_lc != "openclaw" && provider_lc != "ollama" && api_key.is_empty() {
        return Json(json!({"error": "Vision API key missing"}));
    }

    let send_fut = async {
        match provider_lc.as_str() {
            "openrouter" | "openai" | "groq" | "openclaw" => {
                let url = match provider_lc.as_str() {
                    "openrouter" => "https://openrouter.ai/api/v1/chat/completions",
                    "openai" => "https://api.openai.com/v1/chat/completions",
                    "groq" => "https://api.groq.com/openai/v1/chat/completions",
                    _ => openclaw_endpoint.as_str(),
                };
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(reqwest::header::CONTENT_TYPE, "application/json".parse().unwrap());
                if !(provider_lc == "openclaw" && api_key.is_empty()) {
                    headers.insert(reqwest::header::AUTHORIZATION, format!("Bearer {}", api_key).parse().unwrap());
                }
                let body = json!({
                    "model": model,
                    "messages": [{
                        "role": "user",
                        "content": [
                            {"type": "text", "text": prompt},
                            {"type": "image_url", "image_url": {"url": data_url}}
                        ]
                    }]
                });
                println!("[capture] sending to {}", url);
                client.post(url).headers(headers).json(&body).send().await
            }
            "ollama" => {
                let url = format!("{}/api/chat", ollama_endpoint.trim_end_matches('/'));
                let body = json!({
                    "model": model,
                    "messages": [{
                        "role": "user",
                        "content": prompt,
                        "images": [b64]
                    }]
                });
                client.post(url).json(&body).send().await
            }
            _ => unreachable!(),
        }
    };

    let resp = match timeout(Duration::from_secs(50), send_fut).await {
        Ok(res) => res,
        Err(_) => return Json(json!({"error": "Request timed out (send/resp)"})),
    };

    let resp = match resp {
        Ok(r) => r,
        Err(e) => return Json(json!({"error": format!("Request failed: {}", e)})),
    };
    let status = resp.status();
    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => return Json(json!({"error": format!("Read body failed: {}", e)})),
    };
    println!("[capture] status={} len={}", status, text.len());
    if !status.is_success() {
        println!("[capture] body sample: {}", &text.chars().take(500).collect::<String>());
        return Json(json!({"error": format!("HTTP {}: {}", status, text)}));
    }
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return Json(json!({"error": format!("Parse failed: {}", e)})),
    };
    let content_node = parsed
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"));

    let content = if let Some(s) = content_node.and_then(|c| c.as_str()) {
        s.to_string()
    } else if let Some(arr) = content_node.and_then(|c| c.as_array()) {
        let parts: Vec<String> = arr
            .iter()
            .filter_map(|item| {
                if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                    Some(t.to_string())
                } else if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                    item.get("text").and_then(|v| v.as_str()).map(|t| t.to_string())
                } else {
                    None
                }
            })
            .collect();
        parts.join(" ")
    } else {
        "".to_string()
    };
    Json(json!({"text": content, "raw": parsed}))
}

fn ensure_vision_install_dir(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("failed to create vision dir {:?}: {}", dir, e))
}

fn vision_install_marker(dir: &Path) -> PathBuf {
    dir.join("VISION_OK")
}

fn create_venv(dir: &Path) -> Result<PathBuf, String> {
    let venv_dir = dir.join("venv");
    if !venv_dir.exists() {
        let status = Command::new("python")
            .arg("-m")
            .arg("venv")
            .arg(&venv_dir)
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| format!("venv create failed: {}", e))?;
        if !status.success() {
            return Err(format!("venv create exited with status {:?}", status.code()));
        }
    }
    Ok(venv_dir)
}

fn venv_python(venv_dir: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        venv_dir.join("Scripts").join("python.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        venv_dir.join("bin").join("python3")
    }
}

fn pip_install(python: &Path, packages: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new(python);
    cmd.arg("-m").arg("pip").arg("install");
    for pkg in packages {
        cmd.arg(pkg);
    }
    println!("[pip] running: {:?}", cmd);
    let status = cmd
        .creation_flags(0x08000000)
        .status()
        .map_err(|e| format!("pip install failed: {}", e))?;
    if !status.success() {
        return Err(format!("pip install exited with status {:?}", status.code()));
    }
    Ok(())
}

fn detect_python_tag(python: &Path) -> Option<String> {
    let output = Command::new(python)
        .arg("-c")
        .arg("import sys;print(f'cp{sys.version_info.major}{sys.version_info.minor}')")
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if tag.is_empty() { None } else { Some(tag) }
}

fn torch_install_options(_py_tag: &str) -> Vec<Vec<&'static str>> {
    vec![
        vec!["torch==2.10.0+cu121", "--index-url", "https://download.pytorch.org/whl/cu121"],
        vec!["torch==2.10.0+cpu", "--index-url", "https://download.pytorch.org/whl/cpu"],
        vec!["torch==2.4.1+cu121", "--index-url", "https://download.pytorch.org/whl/cu121"],
        vec!["torch==2.4.1+cpu", "--index-url", "https://download.pytorch.org/whl/cpu"],
        vec!["torch", "--index-url", "https://download.pytorch.org/whl/cpu"],
        vec!["torch"],
    ]
}

fn bootstrap_vision_dependencies(settings: &mut AppSettings) -> Result<(), String> {
    let install_dir = PathBuf::from(settings.vision_install_dir.clone());
    ensure_vision_install_dir(&install_dir)?;

    let marker = vision_install_marker(&install_dir);
    // Always ensure latest script/requirements are present
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .ok_or_else(|| "failed to resolve exe dir".to_string())?;

    // Write requirements
    let req_src = exe_dir.join("vision_requirements.txt");
    let req_path = install_dir.join("vision_requirements.txt");
    if req_src.exists() {
        fs::copy(&req_src, &req_path).map_err(|e| format!("failed to copy vision requirements: {}", e))?;
    } else {
        fs::write(&req_path, VISION_REQS_CONTENT).map_err(|e| format!("failed to write vision requirements: {}", e))?;
    }

    // Write script
    let script_dest = install_dir.join("screen_vision_service.py");
    fs::write(&script_dest, VISION_SCRIPT_CONTENT)
        .map_err(|e| format!("failed to overwrite vision script: {}", e))?;

    let venv_dir = create_venv(&install_dir)?;
    let python = venv_python(&venv_dir);

    // Upgrade pip
    let _ = pip_install(&python, &["--upgrade", "pip", "setuptools", "wheel"]);

    pip_install(&python, &["-r", req_path.to_string_lossy().as_ref()])?;

    let py_tag = detect_python_tag(&python).unwrap_or_else(|| "cp311".to_string());
    println!("[VISION] Detected python tag: {}", py_tag);
    let mut torch_ok = false;
    for opts in torch_install_options(&py_tag) {
        println!("[VISION] Trying torch install: {:?}", opts);
        if pip_install(&python, &opts).is_ok() {
            torch_ok = true;
            break;
        }
    }
    if !torch_ok {
        println!("[VISION] Torch install failed for tag {}; moondream may not load", py_tag);
    }

    // Script already written above

    // Write marker
    let mut f = fs::File::create(&marker)
        .map_err(|e| format!("failed to create marker: {}", e))?;
    f.write_all(b"ok").map_err(|e| format!("failed to write marker: {}", e))?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct VisionLatestResponse {
    ts: Option<f64>,
    summary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CaptureAskRequest {
    question: Option<String>,
}

fn start_vision_service(settings: &AppSettings) -> Result<(), String> {
    let install_dir = PathBuf::from(settings.vision_install_dir.clone());
    let py = install_dir.join("venv").join("Scripts").join("python.exe");
    let script = install_dir.join("screen_vision_service.py");

    if !py.exists() {
        return Err(format!("vision python not found at {:?}", py));
    }
    if !script.exists() {
        return Err(format!("vision script not found at {:?}", script));
    }

    // Determine effective provider/model/key based on overrides or LLM settings
    let provider = if !settings.vision_api_type.trim().is_empty() {
        settings.vision_api_type.trim().to_string()
    } else {
        settings.api_type.trim().to_string()
    };

    let model = if !settings.vision_model.trim().is_empty() {
        settings.vision_model.trim().to_string()
    } else {
        match provider.as_str() {
            "ollama" => settings.ollama_model.trim().to_string(),
            "groq" => settings.groq_model.trim().to_string(),
            "openrouter" => settings.openrouter_provider.trim().to_string(),
            "openai" => settings.openrouter_provider.trim().to_string(),
            _ => settings.ollama_model.trim().to_string(),
        }
    };

    let api_key = if !settings.vision_api_key.trim().is_empty() {
        settings.vision_api_key.trim().to_string()
    } else {
        match provider.as_str() {
            "groq" => settings.groq_api_key.trim().to_string(),
            "openrouter" => settings.openrouter_api_key.trim().to_string(),
            "openai" => settings.openrouter_api_key.trim().to_string(),
            _ => String::new(),
        }
    };

    let ollama_endpoint = if !settings.ollama_endpoint.trim().is_empty() {
        settings.ollama_endpoint.trim().to_string()
    } else {
        "http://127.0.0.1:11434".to_string()
    };

    let openclaw_endpoint = if !settings.openclaw_endpoint.trim().is_empty() {
        settings.openclaw_endpoint.trim().to_string()
    } else {
        "http://127.0.0.1:18789".to_string()
    };

    let mut cmd = Command::new(py);
    cmd.arg(script)
        .arg("--port").arg(settings.vision_port.to_string())
        .arg("--interval").arg(format!("{:.2}", settings.vision_interval))
        .arg("--width").arg(settings.vision_width.to_string())
        .arg("--format").arg("webp")
        .arg("--quality").arg("75")
        .creation_flags(0x08000000);

    // propagate vision provider env
    cmd.env("VISION_API_TYPE", provider)
        .env("VISION_MODEL", model)
        .env("VISION_API_KEY", api_key)
        .env("VISION_OLLAMA_ENDPOINT", ollama_endpoint)
        .env("VISION_OPENCLAW_ENDPOINT", openclaw_endpoint);

    println!("[VISION] Starting service: {:?}", cmd);
    cmd.spawn()
        .map_err(|e| format!("Failed to start vision service: {}", e))?;
    Ok(())
}

static VISION_START_ATTEMPTED: AtomicBool = AtomicBool::new(false);

async fn fetch_vision_latest(settings: &AppSettings) -> Option<String> {
    let url = format!("http://127.0.0.1:{}/latest", settings.vision_port);
    let client = HttpClient::new();
    match client.get(url).send().await {
        Ok(resp) => match resp.json::<VisionLatestResponse>().await {
            Ok(data) => data.summary,
            Err(e) => {
                println!("[VISION] failed to parse latest: {}", e);
                None
            }
        },
        Err(e) => {
            println!("[VISION] latest request failed: {}", e);
            if !VISION_START_ATTEMPTED.swap(true, Ordering::SeqCst) {
                if let Err(start_err) = start_vision_service(settings) {
                    println!("[VISION] Autostart on-demand failed: {}", start_err);
                } else {
                    println!("[VISION] Autostart on-demand triggered");
                }
            }
            None
        }
    }
}

// HTTP handler for Groq chat (OpenAI-compatible)
async fn handle_groq_chat(
    Json(payload): Json<GroqChatRequest>,
) -> Json<serde_json::Value> {
    println!("[Groq] ========== CHAT REQUEST START ==========");
    let t_start = Instant::now();

    let settings = load_settings();
    let mut payload = payload;
    if let Some(summary) = fetch_vision_latest(&settings).await {
        println!("[Groq] injecting screen_summary");
        payload.messages.push(OllamaApiMessage {
            role: "system".to_string(),
            content: format!("Screen summary: {}", summary),
        });
    }
    let api_key = settings.groq_api_key.trim().to_string();
    let model = payload
        .model
        .unwrap_or_else(|| settings.groq_model.clone())
        .trim()
        .to_string();

    let msg_count = payload.messages.len();
    let first_preview = payload
        .messages
        .get(0)
        .and_then(|m| Some(m.content.chars().take(120).collect::<String>()))
        .unwrap_or_default();
    println!("[Groq] model='{}' messages={} first='{}'", model, msg_count, first_preview);

    if api_key.is_empty() {
        println!("[Groq] ✗ Missing API key");
        return Json(serde_json::json!({"error": "Groq API key is missing"}));
    }
    if model.is_empty() {
        println!("[Groq] ✗ Missing model");
        return Json(serde_json::json!({"error": "Groq model is missing"}));
    }

    let req_body = serde_json::json!({
        "model": model,
        "messages": payload.messages,
    });

    let client = reqwest::Client::new();
    let url = "https://api.groq.com/openai/v1/chat/completions";

    let t_request = Instant::now();
    match client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&req_body)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    let dt = t_request.elapsed().as_millis();
                    println!("[Groq] Response status: {} ({} ms)", status, dt);
                    let content = data
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string());
                    let mut enriched = data.clone();
                    if let Some(text) = content {
                        let preview: String = text.chars().take(120).collect();
                        println!("[Groq] text len={} preview='{}'", text.len(), preview);
                        enriched["text"] = serde_json::Value::String(text);
                    }
                    println!("[Groq] done in {} ms", t_start.elapsed().as_millis());
                    Json(enriched)
                }
                Err(e) => {
                    println!("[Groq] ✗ Parse error: {}", e);
                    Json(serde_json::json!({"error": format!("Failed to parse response: {}", e)}))
                }
            }
        }
        Err(e) => {
            let dt = t_request.elapsed().as_millis();
            println!("[Groq] ✗ Connection error ({} ms): {}", dt, e);
            Json(serde_json::json!({"error": format!("Failed to reach Groq: {}", e)}))
        }
    }
}

fn ensure_ort_dylib() {
    if env::var("ORT_DYLIB_PATH").is_ok() {
        return;
    }

    const DEFAULT_DLL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/onnxruntime/onnxruntime.dll");
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let candidates = [
        PathBuf::from(DEFAULT_DLL),
        exe_dir.join("resources/onnxruntime/onnxruntime.dll"),
        exe_dir.join("../resources/onnxruntime/onnxruntime.dll"),
        exe_dir.join("../../resources/onnxruntime/onnxruntime.dll"),
        exe_dir.join("../../../resources/onnxruntime/onnxruntime.dll"),
        exe_dir.join("../../../../resources/onnxruntime/onnxruntime.dll"),
        PathBuf::from("S:/OpenClaw/OllamaGUI/local-companion/src-tauri/resources/onnxruntime/onnxruntime.dll"),
    ];

    if let Some(dll) = candidates.into_iter().find(|p| p.exists()) {
        let _ = env::set_var("ORT_DYLIB_PATH", dll);
    }
}

// HTTP server handler for POST /openrouter-chat
async fn handle_openrouter_chat(
    Json(payload): Json<OpenRouterChatRequest>,
) -> Json<serde_json::Value> {
    println!("[OpenRouter] ========== CHAT REQUEST START ==========");

    let settings = load_settings();
    let api_key = settings.openrouter_api_key;
    let model = payload
        .model
        .unwrap_or_else(|| settings.openrouter_provider.clone())
        .trim()
        .to_string();

    if api_key.trim().is_empty() {
        println!("[OpenRouter] ✗ Missing API key");
        return Json(serde_json::json!({"error": "OpenRouter API key is missing"}));
    }

    if model.is_empty() {
        println!("[OpenRouter] ✗ Missing model/provider");
        return Json(serde_json::json!({"error": "OpenRouter model/provider is missing"}));
    }

    // Build OpenRouter payload
    let req_body = serde_json::json!({
        "model": model,
        "messages": payload.messages,
    });

    let client = reqwest::Client::new();
    let url = "https://openrouter.ai/api/v1/chat/completions";

    match client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("HTTP-Referer", "https://openclaw.local")
        .header("X-Title", "OpenClaw Companion")
        .json(&req_body)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    println!("[OpenRouter] Response status: {}", status);
                    // Try to surface a 'text' field for the frontend
                    let content = data
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string());
                    let mut enriched = data.clone();
                    if let Some(text) = content {
                        enriched["text"] = serde_json::Value::String(text);
                    }
                    println!("[OpenRouter] Payload processed successfully");
                    Json(enriched)
                }
                Err(e) => {
                    println!("[OpenRouter] ✗ Parse error: {}", e);
                    Json(serde_json::json!({"error": format!("Failed to parse response: {}", e)}))
                }
            }
        }
        Err(e) => {
            println!("[OpenRouter] ✗ Connection error: {}", e);
            Json(serde_json::json!({"error": format!("Failed to reach OpenRouter: {}", e)}))
        }
    }
}

fn cosine_similarity(a: &ArrayView2<f32>, b: &ArrayView2<f32>) -> f32 {
    let dot = (a * b).sum();
    let na = (a * a).sum().sqrt().max(1e-6);
    let nb = (b * b).sum().sqrt().max(1e-6);
    dot / (na * nb)
}

fn mean_pool(last_hidden: &Array3<f32>, mask: &Array2<i64>) -> Array1<f32> {
    let mask_f = mask.mapv(|v| v as f32);
    let expanded = mask_f.mapv(|v| if v > 0.0 { 1.0 } else { 0.0 });
    let sum: Array1<f32> = last_hidden.index_axis(Axis(0), 0)
        .to_owned()
        .t()
        .dot(&expanded.index_axis(Axis(0), 0));
    let count = expanded.sum() as f32;
    let count = if count < 1.0 { 1.0 } else { count };
    sum / count
}

fn embed_text(tokenizer: &Tokenizer, session: &mut Session, text: &str, max_len: usize) -> Result<Array1<f32>, String> {
    let encoding = tokenizer.encode(text, true).map_err(|e| e.to_string())?;
    let mut ids = encoding.get_ids().to_vec();
    let mut mask = encoding.get_attention_mask().to_vec();
    let mut types = encoding.get_type_ids().to_vec();
    if ids.len() > max_len { ids.truncate(max_len); mask.truncate(max_len); types.truncate(max_len); }
    while ids.len() < max_len { ids.push(0); mask.push(0); types.push(0); }

    let shape = [1usize, max_len];
    let ids_i64: Vec<i64> = ids.into_iter().map(|v| v as i64).collect();
    let mask_i64: Vec<i64> = mask.into_iter().map(|v| v as i64).collect();
    let types_i64: Vec<i64> = types.into_iter().map(|v| v as i64).collect();

    let attention_mask = Array2::from_shape_vec(shape, mask_i64.clone()).map_err(|e| e.to_string())?;

    let v_input_ids = Tensor::from_array((shape, ids_i64)).map_err(|e| e.to_string())?.into_dyn();
    let v_attention = Tensor::from_array((shape, mask_i64)).map_err(|e| e.to_string())?.into_dyn();
    let v_token_types = Tensor::from_array((shape, types_i64)).map_err(|e| e.to_string())?.into_dyn();

    let inputs = vec![
        ("input_ids".to_string(), v_input_ids),
        ("attention_mask".to_string(), v_attention),
        ("token_type_ids".to_string(), v_token_types),
    ];

    let outputs = session
        .run(inputs)
        .map_err(|e| e.to_string())?;
    let first = outputs.into_iter().next().ok_or_else(|| "No outputs from model".to_string())?;
    let last_value = first.1;
    let (shape, data) = last_value
        .try_extract_tensor()
        .map_err(|e| format!("extract error: {e}"))?;
    let shape_vec: Vec<usize> = shape.iter().map(|d| *d as usize).collect();
    let last_arr = Array3::from_shape_vec((shape_vec[0], shape_vec[1], shape_vec[2]), data.to_vec()).map_err(|e| e.to_string())?;
    let pooled = mean_pool(&last_arr, &attention_mask);
    Ok(pooled)
}

fn ensure_emotion() -> Result<(), String> {
    // Initialize ORT environment (process-global). Safe to call multiple times.
    ensure_ort_dylib();
    let _ = init().commit();
    let (model_path, tok_path) = emotion_paths()?;
    EMO_TOKENIZER.get_or_try_init(|| {
        Tokenizer::from_file(tok_path).map_err(|e| e.to_string())
    })?;
    EMO_SESSION.get_or_try_init(|| {
        let session = Session::builder()
            .map_err(|e| e.to_string())?
            .with_intra_threads(1)
            .map_err(|e| e.to_string())?
            .commit_from_file(model_path)
            .map_err(|e| e.to_string())?;
        Ok::<StdMutex<Session>, String>(StdMutex::new(session))
    })?;

    EMO_PROTOS.get_or_try_init(|| {
        let tokenizer = EMO_TOKENIZER.get().unwrap();
        let session_mutex = EMO_SESSION.get().unwrap();
        let mut session = session_mutex.lock().map_err(|e| e.to_string())?;
        let mut map: HashMap<String, Array1<f32>> = HashMap::new();
        for (label, texts) in proto_texts() {
            let mut vecs: Vec<Array1<f32>> = Vec::new();
            for t in texts {
                if let Ok(v) = embed_text(tokenizer, &mut session, t, 128) {
                    vecs.push(v);
                }
            }
            if vecs.is_empty() { continue; }
            let mut sum = vecs[0].clone();
            for v in vecs.iter().skip(1) { sum = &sum + v; }
            let avg = sum / (vecs.len() as f32);
            map.insert(label.to_string(), avg);
        }
        Ok::<HashMap<String, Array1<f32>>, String>(map)
    })?;

    Ok(())
}

fn classify_emotion(text: &str) -> Result<String, String> {
    ensure_emotion()?;
    let tokenizer = EMO_TOKENIZER.get().unwrap();
    let session_mutex = EMO_SESSION.get().unwrap();
    let mut session = session_mutex.lock().map_err(|e| e.to_string())?;
    let protos = EMO_PROTOS.get().unwrap();

    let embed = embed_text(tokenizer, &mut session, text, 128)?;
    let embed2 = embed.insert_axis(Axis(0));
    let mut best = ("neutral".to_string(), -2.0f32);
    for (label, proto) in protos.iter() {
        let proto2 = proto.clone().insert_axis(Axis(0));
        let sim = cosine_similarity(&embed2.view(), &proto2.view());
        if sim > best.1 {
            best = (label.clone(), sim);
        }
    }
    Ok(best.0)
}

fn emotion_paths() -> Result<(PathBuf, PathBuf), String> {
    // Allow manual override via env
    if let (Some(mp), Some(tp)) = (
        std::env::var_os("OPENCLAW_EMOTION_MODEL"),
        std::env::var_os("OPENCLAW_EMOTION_TOKENIZER"),
    ) {
        let mp = PathBuf::from(mp);
        let tp = PathBuf::from(tp);
        if mp.exists() && tp.exists() {
            return Ok((mp, tp));
        }
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    // Compile-time defaults (works in dev/build without env)
    const DEFAULT_MODEL: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/emotion/model_qint8_avx512_vnni.onnx");
    const DEFAULT_TOKENIZER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/emotion/tokenizer.json");

    let candidates = [
        PathBuf::from(DEFAULT_MODEL),
        exe_dir.join("resources/emotion/model_qint8_avx512_vnni.onnx"),
        exe_dir.join("../resources/emotion/model_qint8_avx512_vnni.onnx"),
        exe_dir.join("../../resources/emotion/model_qint8_avx512_vnni.onnx"),
        exe_dir.join("../../../resources/emotion/model_qint8_avx512_vnni.onnx"),
        exe_dir.join("../../../../resources/emotion/model_qint8_avx512_vnni.onnx"),
        PathBuf::from("src-tauri/resources/emotion/model_qint8_avx512_vnni.onnx"),
        PathBuf::from("resources/emotion/model_qint8_avx512_vnni.onnx"),
        PathBuf::from("S:/OpenClaw/OllamaGUI/local-companion/src-tauri/resources/emotion/model_qint8_avx512_vnni.onnx"),
    ];

    let tok_candidates = [
        PathBuf::from(DEFAULT_TOKENIZER),
        exe_dir.join("resources/emotion/tokenizer.json"),
        exe_dir.join("../resources/emotion/tokenizer.json"),
        exe_dir.join("../../resources/emotion/tokenizer.json"),
        exe_dir.join("../../../resources/emotion/tokenizer.json"),
        exe_dir.join("../../../../resources/emotion/tokenizer.json"),
        PathBuf::from("src-tauri/resources/emotion/tokenizer.json"),
        PathBuf::from("resources/emotion/tokenizer.json"),
        PathBuf::from("S:/OpenClaw/OllamaGUI/local-companion/src-tauri/resources/emotion/tokenizer.json"),
    ];

    let model_path = candidates.into_iter().find(|p| p.exists()).ok_or_else(|| "Model file not found (emotion)".to_string())?;
    let tok_path = tok_candidates.into_iter().find(|p| p.exists()).ok_or_else(|| "Tokenizer file not found (emotion)".to_string())?;
    Ok((model_path, tok_path))
}

// HTTP handler: local emotion inference
async fn handle_emotion(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    if let Some(text) = payload.get("text").and_then(|t| t.as_str()) {
        match classify_emotion(text) {
            Ok(emotion) => Json(serde_json::json!({ "emotion": emotion })),
            Err(e) => Json(serde_json::json!({ "error": e })),
        }
    } else {
        Json(serde_json::json!({ "error": "No text provided" }))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandPayload {
    #[serde(rename = "type")]
    pub cmd_type: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub viseme: Option<String>,
    #[serde(default)]
    pub intensity: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaChatRequest {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub messages: Option<Vec<OllamaApiMessage>>,
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaChatResponse {
    pub text: String,
    pub emotion: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OllamaApiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaApiRequest {
    pub model: String,
    pub messages: Vec<OllamaApiMessage>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaApiResponse {
    pub message: OllamaApiMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaModelsResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenRouterChatRequest {
    pub messages: Vec<OllamaApiMessage>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppSettings {
    pub api_type: String,
    pub ollama_endpoint: String,
    pub ollama_model: String,
    pub openclaw_endpoint: String,
    #[serde(default = "default_openrouter_api_key")]
    pub openrouter_api_key: String,
    #[serde(default = "default_openrouter_provider")]
    pub openrouter_provider: String,
    #[serde(default = "default_groq_api_key")]
    pub groq_api_key: String,
    #[serde(default = "default_groq_model")]
    pub groq_model: String,
    #[serde(default = "default_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_tts_engine")]
    pub tts_engine: String,
    #[serde(default = "default_tts_language")]
    pub tts_language: String,
    #[serde(default = "default_tts_api_key")]
    pub tts_api_key: String,
    #[serde(default = "default_tts_voice_id")]
    pub tts_voice_id: String,
    #[serde(default = "default_custom_vrm_name")]
    pub custom_vrm_name: String,
    #[serde(default = "default_custom_vrm_path")]
    pub custom_vrm_path: String,
    #[serde(default = "default_vision_autostart")]
    pub vision_autostart: bool,
    #[serde(default = "default_vision_port")]
    pub vision_port: u16,
    #[serde(default = "default_vision_interval")]
    pub vision_interval: f32,
    #[serde(default = "default_vision_width")]
    pub vision_width: u32,
    #[serde(default = "default_vision_install_dir")]
    pub vision_install_dir: String,
    #[serde(default = "default_vision_api_type")]
    pub vision_api_type: String,
    #[serde(default = "default_vision_model")]
    pub vision_model: String,
    #[serde(default = "default_vision_api_key")]
    pub vision_api_key: String,
    #[serde(default = "default_use_vision_model")]
    pub use_vision_model: bool,
    #[serde(default = "default_stt_provider")]
    pub stt_provider: String,
    #[serde(default = "default_stt_api_key")]
    pub stt_api_key: String,
    #[serde(default = "default_stt_autostart")]
    pub stt_autostart: bool,
    #[serde(default = "default_stt_port")]
    pub stt_port: u16,
}

fn default_tts_engine() -> String {
    "xtts_v2".to_string()
}

fn default_tts_language() -> String {
    "tr".to_string()
}

fn default_openrouter_api_key() -> String {
    "".to_string()
}

fn default_openrouter_provider() -> String {
    "".to_string()
}

fn default_system_prompt() -> String {
    "".to_string()
}

fn default_tts_api_key() -> String {
    "".to_string()
}

fn default_tts_voice_id() -> String {
    "".to_string()
}

fn default_custom_vrm_name() -> String {
    "".to_string()
}

fn default_custom_vrm_path() -> String {
    "".to_string()
}

fn default_groq_api_key() -> String {
    "".to_string()
}

fn default_groq_model() -> String {
    String::new()
}

fn default_vision_autostart() -> bool {
    true
}

fn default_vision_port() -> u16 {
    8777
}

fn default_vision_interval() -> f32 {
    2.0
}

fn default_vision_width() -> u32 {
    960
}

fn default_vision_install_dir() -> String {
    if let Ok(appdata) = env::var("APPDATA") {
        let mut base = PathBuf::from(appdata);
        base.push("OllamaGUI");
        base.push("Vision");
        return base.to_string_lossy().to_string();
    }
    "Vision".to_string()
}

fn default_vision_api_type() -> String { "".to_string() }
fn default_vision_model() -> String { "".to_string() }
fn default_vision_api_key() -> String { "".to_string() }
fn default_use_vision_model() -> bool { true }
fn default_stt_provider() -> String { "deepgram".to_string() }
fn default_stt_api_key() -> String { "".to_string() }
fn default_stt_autostart() -> bool { true }
fn default_stt_port() -> u16 { 5001 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_type: "ollama".to_string(),
            ollama_endpoint: String::new(),
            ollama_model: String::new(),
            openclaw_endpoint: String::new(),
            openrouter_api_key: default_openrouter_api_key(),
            openrouter_provider: default_openrouter_provider(),
            groq_api_key: default_groq_api_key(),
            groq_model: default_groq_model(),
            system_prompt: default_system_prompt(),
            tts_engine: default_tts_engine(),
            tts_language: default_tts_language(),
            tts_api_key: default_tts_api_key(),
            tts_voice_id: default_tts_voice_id(),
            custom_vrm_name: default_custom_vrm_name(),
            custom_vrm_path: default_custom_vrm_path(),
            vision_autostart: default_vision_autostart(),
            vision_port: default_vision_port(),
            vision_interval: default_vision_interval(),
            vision_width: default_vision_width(),
            vision_install_dir: default_vision_install_dir(),
            vision_api_type: default_vision_api_type(),
            vision_model: default_vision_model(),
            vision_api_key: default_vision_api_key(),
            use_vision_model: default_use_vision_model(),
            stt_provider: default_stt_provider(),
            stt_api_key: default_stt_api_key(),
            stt_autostart: default_stt_autostart(),
            stt_port: default_stt_port(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroqChatRequest {
    pub messages: Vec<OllamaApiMessage>,
    #[serde(default)]
    pub model: Option<String>,
}

#[tauri::command]
async fn speak_edge_tts(text: String, voice: String) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let output_path = temp_dir.join("openclaw_tts.mp3");
    let output_str = output_path.to_string_lossy().to_string();

    let result = Command::new("python")
        .args([
            "-m", "edge_tts",
            "--voice", &voice,
            "--text", &text,
            "--write-media", &output_str,
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW on Windows
        .output()
        .map_err(|e| format!("Failed to run edge-tts: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("edge-tts error: {}", stderr));
    }

    // Read file and return as base64 data URI
    let bytes = std::fs::read(&output_path)
        .map_err(|e| format!("Failed to read audio file: {}", e))?;
    
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:audio/mpeg;base64,{}", b64))
}

// Start WebSocket server on localhost:9001
async fn start_websocket_server(clients: SharedClients) {
    let addr = "127.0.0.1:9001";

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            println!("[WebSocket] Server listening on ws://{}", addr);
            let mut client_counter = 0;

            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let clients = clients.clone();
                        client_counter += 1;
                        let client_id = format!("client_{}", client_counter);

                        tokio::spawn(async move {
                            if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                                println!("[WebSocket] Client connected: {}", client_id);
                                let (ws_sender, mut ws_receiver) = ws_stream.split();
                                let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

                                // Register client
                                clients.lock().await.insert(client_id.clone(), tx);

                                // Spawn task to forward messages from channel to WebSocket
                                let ws_sender = Arc::new(Mutex::new(ws_sender));
                                let ws_sender_clone = ws_sender.clone();
                                tokio::spawn(async move {
                                    while let Some(msg) = rx.recv().await {
                                        let mut sender = ws_sender_clone.lock().await;
                                        let _ = sender.send(msg).await;
                                    }
                                });

                                // Receive messages from client and broadcast
                                while let Some(msg) = ws_receiver.next().await {
                                    if let Ok(msg) = msg {
                                        if let Ok(text) = msg.to_text() {
                                            println!("[WebSocket] Received from {}: {}", client_id, text);
                                            // Broadcast to all clients
                                            let msg_to_send = Message::Text(text.to_string());
                                            let clients_lock = clients.lock().await;
                                            for (_, tx) in clients_lock.iter() {
                                                let _ = tx.send(msg_to_send.clone());
                                            }
                                        }
                                    }
                                }

                                // Unregister client
                                clients.lock().await.remove(&client_id);
                                println!("[WebSocket] Client disconnected: {}", client_id);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("[WebSocket] Accept error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("[WebSocket] Failed to bind: {}", e);
        }
    }
}

// Shared state for HTTP/WebSocket communication
type SharedClients = Arc<Mutex<HashMap<String, tokio::sync::mpsc::UnboundedSender<Message>>>>;

// HTTP server handler for POST /command
async fn handle_command(
    axum::extract::State(clients): axum::extract::State<SharedClients>,
    Json(payload): Json<CommandPayload>,
) -> Json<serde_json::Value> {
    println!("[HTTP] Received command: {:?}", payload);
    
    // Convert HTTP payload to WebSocket message
    let msg_text = serde_json::to_string(&payload).unwrap_or_default();
    let ws_msg = Message::Text(msg_text.clone());
    
    // Broadcast to all WebSocket clients
    let clients_lock = clients.lock().await;
    for (_, tx) in clients_lock.iter() {
        let _ = tx.send(ws_msg.clone());
    }
    drop(clients_lock);
    
    println!("[HTTP] Broadcasted to WebSocket clients: {}", msg_text);
    
    Json(serde_json::json!({
        "status": "ok",
        "message": "Command received and broadcasted"
    }))
}

// HTTP server handler for POST /lilith-chat (proxy to Node.js proxy server)
async fn handle_lilith_chat(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    println!("[Chat] ========== CHAT REQUEST START ==========");
    println!("[Chat] Payload: {:?}", payload);

    // Enrich payload with latest screen summary (if available)
    let mut payload = payload;
    if let Some(mut obj) = payload.as_object().cloned() {
        let settings = load_settings();
        if let Some(summary) = fetch_vision_latest(&settings).await {
            obj.insert("screen_summary".to_string(), serde_json::Value::String(summary));
            payload = serde_json::Value::Object(obj);
        }
    }
    
    // Node.js Proxy Server (localhost:3032)
    // Windows proxy → VPS Lilith Proxy (18789)
    let proxy_url = "http://127.0.0.1:3032/api/command";
    println!("[Chat] Target URL: {}", proxy_url);
    
    let client = reqwest::Client::new();
    println!("[Chat] Sending POST request...");
    
    match client
        .post(proxy_url)
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => {
            println!("[Chat] Response status: {}", response.status());
            
            if !response.status().is_success() {
                println!("[Chat] HTTP error: {}", response.status());
                return Json(serde_json::json!({
                    "error": format!("Proxy error: {}", response.status())
                }));
            }
            
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    println!("[Chat] ✓ Response received: {:?}", data);
                    println!("[Chat] ========== CHAT REQUEST END ==========");
                    Json(data)
                }
                Err(e) => {
                    println!("[Chat] ✗ Parse error: {}", e);
                    println!("[Chat] ========== CHAT REQUEST END ==========");
                    Json(serde_json::json!({
                        "error": format!("Failed to parse response: {}", e)
                    }))
                }
            }
        }
        Err(e) => {
            println!("[Chat] ✗ Connection error: {}", e);
            println!("[Chat] ========== CHAT REQUEST END ==========");
            Json(serde_json::json!({
                "error": format!("Failed to reach proxy: {}", e)
            }))
        }
    }
}

// HTTP server handler for GET /lilith-health (health check)
async fn handle_lilith_health() -> Json<serde_json::Value> {
    println!("[Lilith] Health check request");
    
    let lilith_url = "https://www.3v4.club/health";
    let client = reqwest::Client::new();
    
    match client.get(lilith_url).send().await {
        Ok(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    println!("[Lilith] Health check: OK");
                    Json(data)
                }
                Err(e) => {
                    println!("[Lilith] Health check parse error: {}", e);
                    Json(serde_json::json!({"status": "error", "message": e.to_string()}))
                }
            }
        }
        Err(e) => {
            println!("[Lilith] Health check connection error: {}", e);
            Json(serde_json::json!({"status": "error", "message": e.to_string()}))
        }
    }
}

// HTTP server handler for GET /lilith-status (status check)
async fn handle_lilith_status() -> Json<serde_json::Value> {
    println!("[Lilith] Status request");
    
    let lilith_url = "https://www.3v4.club/api/v1/lilith/status";
    let client = reqwest::Client::new();
    
    match client.get(lilith_url).send().await {
        Ok(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    println!("[Lilith] Status: {:?}", data);
                    Json(data)
                }
                Err(e) => {
                    println!("[Lilith] Status parse error: {}", e);
                    Json(serde_json::json!({"status": "error", "message": e.to_string()}))
                }
            }
        }
        Err(e) => {
            println!("[Lilith] Status connection error: {}", e);
            Json(serde_json::json!({"status": "error", "message": e.to_string()}))
        }
    }
}

// HTTP server handler for POST /lilith-emotion (emotion update)
async fn handle_lilith_emotion(
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    println!("[Lilith] Emotion update request: {:?}", payload);
    
    let lilith_url = "https://www.3v4.club/api/v1/lilith/emotion";
    let client = reqwest::Client::new();
    
    match client.post(lilith_url).json(&payload).send().await {
        Ok(response) => {
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    println!("[Lilith] Emotion update: OK");
                    Json(data)
                }
                Err(e) => {
                    println!("[Lilith] Emotion update parse error: {}", e);
                    Json(serde_json::json!({"status": "error", "message": e.to_string()}))
                }
            }
        }
        Err(e) => {
            println!("[Lilith] Emotion update connection error: {}", e);
            Json(serde_json::json!({"status": "error", "message": e.to_string()}))
        }
    }
}

// Analyze emotion from text using Ollama
async fn analyze_emotion_with_ollama(
    text: &str,
    ollama_endpoint: &str,
    model: &str,
) -> String {
    let emotion_prompt = format!(
        "Analyze the emotion in this text and respond with ONLY one word from this list: happy, sad, angry, neutral, surprised, relaxed. Text: \"{}\"",
        text
    );

    let request = OllamaApiRequest {
        model: model.to_string(),
        messages: vec![OllamaApiMessage {
            role: "user".to_string(),
            content: emotion_prompt,
        }],
        stream: false,
    };

    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", ollama_endpoint);

    match client.post(&url).json(&request).send().await {
        Ok(response) => {
            match response.json::<OllamaApiResponse>().await {
                Ok(data) => {
                    let emotion = data.message.content.trim().to_lowercase();
                    let valid_emotions = vec!["happy", "sad", "angry", "neutral", "surprised", "relaxed"];
                    
                    if valid_emotions.contains(&emotion.as_str()) {
                        emotion
                    } else {
                        "neutral".to_string()
                    }
                }
                Err(_) => "neutral".to_string(),
            }
        }
        Err(_) => "neutral".to_string(),
    }
}

// Get settings file path
fn get_settings_path() -> std::path::PathBuf {
    let app_data = std::env::var("APPDATA")
        .unwrap_or_else(|_| std::env::var("HOME").unwrap_or_else(|_| ".".to_string()));
    let settings_dir = std::path::Path::new(&app_data).join("OllamaGUI");
    let _ = std::fs::create_dir_all(&settings_dir);
    settings_dir.join("settings.json")
}

// Load settings from file
fn load_settings() -> AppSettings {
    let path = get_settings_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            match serde_json::from_str::<AppSettings>(&content) {
                Ok(settings) => {
                    println!("[Settings] Loaded from: {:?}", path);
                    settings
                }
                Err(_) => {
                    println!("[Settings] Invalid JSON, using defaults");
                    AppSettings::default()
                }
            }
        }
        Err(_) => {
            println!("[Settings] File not found, using defaults");
            AppSettings::default()
        }
    }
}

// Save settings to file
fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let path = get_settings_path();
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings: {}", e))?;
    
    println!("[Settings] Saved to: {:?}", path);
    Ok(())
}

// Tauri command: Get current settings
#[tauri::command]
fn get_settings() -> AppSettings {
    load_settings()
}

// Tauri command: Update settings
#[tauri::command]
fn update_settings(settings: AppSettings) -> Result<AppSettings, String> {
    save_settings(&settings)?;
    Ok(settings)
}

// Tauri command: Get available models from Ollama
#[tauri::command]
async fn get_ollama_models_handler(endpoint: String) -> Result<Vec<String>, String> {
    println!("[Models] Fetching from: {}", endpoint);
    
    // Normalize endpoint - remove trailing slash and /v1 if present
    let endpoint = endpoint.trim_end_matches('/');
    let endpoint = if endpoint.ends_with("/v1") {
        endpoint.trim_end_matches("/v1").to_string()
    } else {
        endpoint.to_string()
    };
    
    let url = format!("{}/api/tags", endpoint);
    println!("[Models] Full URL: {}", url);
    
    let client = reqwest::Client::new();
    
    match client.get(&url).timeout(std::time::Duration::from_secs(10)).send().await {
        Ok(response) => {
            println!("[Models] Response status: {}", response.status());
            match response.json::<serde_json::Value>().await {
                Ok(data) => {
                    if let Some(models) = data.get("models").and_then(|m| m.as_array()) {
                        let model_names: Vec<String> = models
                            .iter()
                            .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                            .collect();
                        println!("[Models] Found {} models", model_names.len());
                        Ok(model_names)
                    } else {
                        Err("No models field in response".to_string())
                    }
                }
                Err(e) => {
                    println!("[Models] JSON parse error: {}", e);
                    Err(format!("Failed to parse response: {}", e))
                }
            }
        }
        Err(e) => {
            println!("[Models] Connection error: {}", e);
            Err(format!("Failed to connect to Ollama: {}", e))
        }
    }
}

// HTTP server handler for POST /ollama-chat
async fn handle_ollama_chat(
    Json(payload): Json<OllamaChatRequest>,
) -> Json<OllamaChatResponse> {
    println!("[Ollama] ========== CHAT REQUEST START ==========");

    // Load settings to get the correct endpoint
    let settings = load_settings();
    let ollama_endpoint = if settings.ollama_endpoint.trim().is_empty() {
        "http://127.0.0.1:11434".to_string()
    } else {
        settings.ollama_endpoint.trim().to_string()
    };
    let model = payload.model.unwrap_or_else(|| settings.ollama_model.clone());

    println!("[Ollama] Endpoint: {}", ollama_endpoint);
    println!("[Ollama] Model: {}", model);

    // Use messages array if provided, otherwise use single message
    let messages = if let Some(msgs) = payload.messages {
        println!("[Ollama] Using message history: {} messages", msgs.len());
        msgs
    } else if let Some(msg) = payload.message {
        println!("[Ollama] Using single message: {}", msg);
        vec![OllamaApiMessage {
            role: "user".to_string(),
            content: msg,
        }]
    } else {
        println!("[Ollama] No message provided");
        return Json(OllamaChatResponse {
            text: "No message provided".to_string(),
            emotion: "neutral".to_string(),
        });
    };

    let request = OllamaApiRequest {
        model: model.clone(),
        messages,
        stream: false,
    };

    let client = reqwest::Client::new();
    // Use Ollama native /api/chat endpoint instead of /chat/completions
    let url = format!("{}/api/chat", ollama_endpoint);

    // Retry logic - try up to 3 times
    let mut retry_count = 0;
    let max_retries = 2;
    
    loop {
        match client.post(&url).json(&request).send().await {
        Ok(response) => {
            // First get response text for debugging
            let response_text_raw = match response.text().await {
                Ok(text) => {
                    println!("[Ollama] Raw response: {}", text);
                    text
                }
                Err(e) => {
                    println!("[Ollama] ✗ Failed to read response: {}", e);
                    return Json(OllamaChatResponse {
                        text: format!("Failed to read response: {}", e),
                        emotion: "neutral".to_string(),
                    });
                }
            };

            // Now parse as JSON
            match serde_json::from_str::<serde_json::Value>(&response_text_raw) {
                Ok(data) => {
                    // Try to extract response text from different possible formats
                    let response_text = if let Some(msg) = data.get("message") {
                        // Ollama native format: {message: {content: "..."}}
                        msg.get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("No response")
                            .to_string()
                    } else if let Some(choices) = data.get("choices") {
                        // OpenAI format: {choices: [{message: {content: "..."}}]}
                        choices
                            .get(0)
                            .and_then(|c| c.get("message"))
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("No response")
                            .to_string()
                    } else {
                        "No response".to_string()
                    };

                    let (clean_text, inferred_emotion) = clean_ollama_response(&response_text);
                    println!("[Ollama] Response text: {}", clean_text);

                    let emotion = inferred_emotion;
                    println!("[Ollama] Emotion: {}", emotion);
                    println!("[Ollama] ========== CHAT REQUEST END ==========");

                    return Json(OllamaChatResponse {
                        text: clean_text,
                        emotion,
                    });
                }
                Err(e) => {
                    println!("[Ollama] ✗ Parse error: {}", e);
                    println!("[Ollama] ========== CHAT REQUEST END ==========");
                    return Json(OllamaChatResponse {
                        text: format!("Error parsing response: {}", e),
                        emotion: "neutral".to_string(),
                    });
                }
            }
        }
        Err(e) => {
            if retry_count < max_retries {
                retry_count += 1;
                println!("[Ollama] ⚠ Connection error, retrying ({}/{}): {}", retry_count, max_retries, e);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            } else {
                println!("[Ollama] ✗ Connection error (all retries failed): {}", e);
                println!("[Ollama] ========== CHAT REQUEST END ==========");
                return Json(OllamaChatResponse {
                    text: format!("Error connecting to Ollama: {}", e),
                    emotion: "neutral".to_string(),
                });
            }
        }
        }
    }
}

// HTTP server handler for GET /settings
async fn handle_get_settings() -> Json<AppSettings> {
    println!("[Settings] Loading settings...");
    Json(load_settings())
}

// HTTP server handler for POST /settings
async fn handle_update_settings(
    Json(payload): Json<AppSettings>,
) -> Json<AppSettings> {
    println!("[Settings] Saving settings: {:?}", payload);
    if let Err(e) = save_settings(&payload) {
        eprintln!("[Settings] Error saving: {}", e);
    }
    Json(payload)
}

// HTTP server handler for POST /ollama-models
async fn handle_ollama_models(
    Json(payload): Json<serde_json::Value>,
) -> Json<Vec<String>> {
    println!("[Models] Fetching models from Ollama...");
    
    if let Some(endpoint) = payload.get("endpoint").and_then(|e| e.as_str()) {
        println!("[Models] Endpoint: {}", endpoint);
        
        // Normalize endpoint
        let endpoint = endpoint.trim_end_matches('/');
        let endpoint = if endpoint.ends_with("/v1") {
            endpoint.trim_end_matches("/v1").to_string()
        } else {
            endpoint.to_string()
        };
        
        let url = format!("{}/api/tags", endpoint);
        let client = reqwest::Client::new();
        
        match client.get(&url).timeout(std::time::Duration::from_secs(10)).send().await {
            Ok(response) => {
                match response.json::<serde_json::Value>().await {
                    Ok(data) => {
                        if let Some(models) = data.get("models").and_then(|m| m.as_array()) {
                            let model_names: Vec<String> = models
                                .iter()
                                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                                .collect();
                            println!("[Models] Found {} models", model_names.len());
                            return Json(model_names);
                        }
                    }
                    Err(e) => {
                        println!("[Models] JSON parse error: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("[Models] Connection error: {}", e);
            }
        }
    }
    
    Json(vec![])
}

// HTTP server handler for POST /speak (TTS)
async fn handle_speak(
    Json(payload): Json<serde_json::Value>,
) -> Result<impl axum::response::IntoResponse, String> {
    if let Some(text) = payload.get("text").and_then(|t| t.as_str()) {
        println!("[TTS] Generating speech for: {}", text.chars().take(50).collect::<String>());
        // Load settings for TTS
        let settings = load_settings();
        let client = reqwest::Client::new();

        if settings.tts_engine == "elevenlabs" {
            // ElevenLabs API
            if settings.tts_api_key.trim().is_empty() {
                return Err("ElevenLabs API key is missing".to_string());
            }
            let voice_id = if settings.tts_voice_id.trim().is_empty() {
                "21m00Tcm4TlvDq8ikWAM".to_string() // default common voice
            } else {
                settings.tts_voice_id.trim().to_string()
            };

            let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
            let body = serde_json::json!({
                "text": text,
                "voice_settings": {
                    "stability": 0.5,
                    "similarity_boost": 0.75,
                    "style": 0.3,
                    "use_speaker_boost": true
                }
            });

            match client
                .post(url)
                .header("xi-api-key", settings.tts_api_key.trim())
                .header("Accept", "audio/mpeg")
                .json(&body)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    if !status.is_success() {
                        let error_body = response.text().await.unwrap_or_default();
                        println!("[TTS] ElevenLabs error ({}): {}", status, error_body);
                        return Err(format!("ElevenLabs error: {} - {}", status, error_body));
                    }
                    match response.bytes().await {
                        Ok(audio_bytes) => {
                            let is_audio = audio_bytes.len() > 1000;
                            println!("[TTS] ElevenLabs bytes: {}, is_audio: {}", audio_bytes.len(), is_audio);
                            Ok((
                                axum::http::StatusCode::OK,
                                [("Content-Type", "audio/mpeg")],
                                audio_bytes,
                            ))
                        }
                        Err(e) => {
                            println!("[TTS] ElevenLabs read error: {}", e);
                            Err(format!("Failed to read ElevenLabs response: {}", e))
                        }
                    }
                }
                Err(e) => {
                    println!("[TTS] Error calling ElevenLabs: {}", e);
                    Err(format!("Failed to call ElevenLabs: {}", e))
                }
            }
        } else {
            Err("Only ElevenLabs TTS is supported. Please provide an API key.".to_string())
        }
    } else {
        Err("No text provided".to_string())
    }
}

// Start HTTP server on localhost:3030
async fn start_http_server(clients: SharedClients) {
    let app = Router::new()
        .route("/command", post(handle_command))
        .route("/lilith-chat", post(handle_lilith_chat))
        .route("/ollama-chat", post(handle_ollama_chat))
        .route("/openrouter-chat", post(handle_openrouter_chat))
        .route("/groq-chat", post(handle_groq_chat))
        .route("/settings", get(handle_get_settings))
        .route("/settings", post(handle_update_settings))
        .route("/ollama-models", post(handle_ollama_models))
        .route("/speak", post(handle_speak))
        .route("/emotion", post(handle_emotion))
        .route("/lilith-health", get(handle_lilith_health))
        .route("/lilith-status", get(handle_lilith_status))
        .route("/lilith-emotion", post(handle_lilith_emotion))
        .route("/capture_and_ask", post(handle_capture_and_ask))
        .layer(CorsLayer::permissive())
        .with_state(clients);

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:3030").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[HTTP] Failed to bind: {}", e);
            return;
        }
    };

    println!("[HTTP] Server listening on http://127.0.0.1:3030");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("[HTTP] Server error: {}", e);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|_app| {
            // Create shared clients state
            let clients: SharedClients = Arc::new(Mutex::new(HashMap::new()));
            let clients_ws = clients.clone();
            let clients_http = clients.clone();
            
            // Start WebSocket server in background using Tauri's runtime
            tauri::async_runtime::spawn(start_websocket_server(clients_ws));
            // Start HTTP server in background
            tauri::async_runtime::spawn(start_http_server(clients_http));
            // Attempt STT autostart
            let settings = load_settings();
            if settings.stt_autostart {
                if let Err(e) = start_stt_daemon(&settings) {
                    println!("[STT] Autostart failed: {}", e);
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![speak_edge_tts, get_settings, update_settings, get_ollama_models_handler])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

<<<<<<< HEAD

<div align="center">

# ✨ V-Lucent
### **The Transparent, Screen-Aware 3D AI Companion**


[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5.0-FF3E00?style=for-the-badge&logo=svelte&logoColor=white)](https://svelte.dev/)
[![Rust](https://img.shields.io/badge/Rust-Stable-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

<!-- <img src="https://via.placeholder.com/800x400?text=Insert+V-Lucent+Demo+GIF+Here" alt="V-Lucent Preview" width="800" style="border-radius: 10px; margin: 20px 0;" /> -->

**V-Lucent** is a high-performance 3D AI assistant that blends a **VRM avatar**, **speech-to-speech interaction**, and advanced **screen-awareness** into a single, distraction-free desktop application.

[Explore Docs](#-quick-start) • [Report Bug](https://github.com/codacipher/v-lucent/issues) • [Request Feature](https://github.com/codacipher/v-lucent/issues)

</div>

---

## 🚀 Highlights

- **🪟 Transparent Overlay** – Native desktop companion that floats with zero background chrome.
- **👁️ Screen Awareness** – Reads live screen context to offer proactive, contextual chat.
- **🗣️ Low-Latency Speech** – Deepgram (STT) ➜ LLM ➜ ElevenLabs (TTS) pipeline.
- **🎭 3D Emotional Presence** – VRoid/VRM rig with emotion-driven animations + visemes.
- **🧠 Brain Switcher** – Hot-swap between **Ollama, OpenClaw, OpenRouter, Groq**.

---
=======
# v-lucent — Local Companion for OpenClaw

A desktop-ready AI companion that blends a VRM avatar, speech-to-speech chat, and OpenClaw’s VPS brain into a single Tauri application. The app runs on Windows today (macOS/Linux coming later) and ships with a slim front-end (`local-companion/`) plus archival tooling under `legacy/` for reference.

## ✨ Highlights

- **Modern stack** – Svelte 5 + Vite UI packaged with Tauri 2 / Rust backend.
- **Speech-to-speech** – Browser audio capture → Deepgram (default) for STT → LLM routing → ElevenLabs TTS playback with viseme-driven lip sync.
- **VRM rendering** – Three.js + @pixiv/three-vrm scene with drag/orbit camera, idle motions, emotion-driven blendshapes, and persisted custom models.
- **Multi-provider LLM bridge** – Ollama, OpenClaw, OpenRouter, and Groq endpoints selectable per session; pluggable system prompt, emotion JSON hints, and optional screen-vision loop.
- **Legacy isolation** – All Docker / proxy / XTTS experiments parked inside `legacy/` so the active app stays lightweight.
>>>>>>> 6839fe2 (Initial commit)

## 📁 Repository Layout

```
<<<<<<< HEAD
V-Lucent/
├── local-companion/         # Active Tauri + Svelte 5 application
│   ├── src/                 # Frontend (VRMRenderer, chat UI, settings)
│   ├── src-tauri/           # Rust backend (commands, proxy, vision)
│   └── public/models/       # Default VRM assets
└── README.md
```

---

## 🛠️ Quick Start

### Prerequisites

- Node.js 20+
- Rust toolchain (stable) + cargo
- Windows 10/11 (WebView2 runtime installed)

### Setup & Run

```bash
# 1. Clone the universe
git clone https://github.com/codacipher/v-lucent.git

# 2. Enter the chamber
cd v-lucent/local-companion

# 3. Install dependencies
npm install

# 4. Launch V-Lucent
npm run tauri dev
```

> **💡 Tip:** Svelte UI runs at `localhost:1420`, while the Rust backend brokers APIs on `localhost:3030`.

---

## ⚙️ Configuration Cheatsheet

| Component | Location | Details |
|-----------|----------|---------|
| **LLM Provider** | Settings → LLM | Switch between Ollama, OpenClaw, OpenRouter, Groq |
| **Custom VRM** | Camera Panel | Drag & drop `.vrm`; stored under `%APPDATA%/local-companion` |
| **Audio Keys** | Settings → Audio | Add Deepgram (STT) + ElevenLabs (TTS) API keys |
| **Vision Assist** | Settings → Visual | Toggle `screen_vision_service.py` for on-screen context |

---

## 🧱 Architecture Snapshot

- **Frontend (Svelte 5):** `VRMRenderer.svelte` drives the Three.js scene, visemes, and Runes-based state.
- **Backend (Rust/Tauri):** Command layer for LLM proxying, native screen capture, and `%APPDATA%/OllamaGUI/settings.json` persistence.
- **Speech Pipeline:** MediaRecorder ➜ Deepgram ➜ LLM ➜ ElevenLabs chunked playback for a lifelike voice loop.
- **Legacy Vault:** Historical XTTS/Docker tooling lives in `/legacy` for devs needing local-only stacks.

---

## 🗺️ Roadmap

- **v1.0** – Tauri 2 / Svelte 5 Core Engine
- **v1.1** – Real-time viseme & lip-sync polish
- **v1.2** – Multi-provider LLM routing + vision loop
- **v2.0** – Twitch/YouTube live mode
- **v2.1** – Long-term memory (vector DB)
- **v2.2** – macOS & Linux builds

---

## 🤝 Contributing

1. Fork the repo
2. Create a feature branch: `git checkout -b feature/AmazingFeature`
3. Commit your changes: `git commit -m "Add AmazingFeature"`
4. Push to the branch: `git push origin feature/AmazingFeature`
5. Open a Pull Request

---

<div align="center">

Let's build the future of desktop companions. 🚀

**Licensed under MIT • Built with ❤️ by CodaCipher**

![Rainbow Line](https://raw.githubusercontent.com/andreasbm/readme/master/assets/lines/rainbow.png)

</div>
=======
OllamaGUI/
├── local-companion/         # Active Tauri + Svelte application
│   ├── src/                 # Frontend (VRMRenderer, chat UI, settings)
│   ├── src-tauri/           # Rust commands, HTTP bridge, vision tools
│   └── public/models/       # Default VRM assets
├── legacy/                  # Archived Docker proxy + integration backend
└── README.md                # ← you are here
```

> ℹ️ Additional architecture / proxy instruction docs live locally (ignored via `.gitignore`). Copy them manually if you need them in releases or a wiki.

## 🚀 Quick Start

1. **Install dependencies**
   - Node.js 20+
   - Rust toolchain (stable) + `cargo`
   - Tauri prerequisites (MSVC build tools, WebView2) → [Tauri checklist](https://tauri.app/v1/guides/getting-started/prerequisites/)

2. **Install packages**
   ```bash
   cd local-companion
   npm install
   ```

3. **Run the desktop app**
   ```bash
   npm run tauri dev
   ```
   The Svelte UI launches at `localhost:1420` inside the native shell; the Tauri backend exposes helper HTTP endpoints on `localhost:3030`.

4. **(Optional) Configure STT/TTS keys**
   - Open the in-app **Settings → STT** section and paste a Deepgram API key (characters are sanitized before use).
   - Add your ElevenLabs API key / voice ID for TTS playback.
   - All secrets persist in `%APPDATA%/OllamaGUI/settings.json`, which is outside the repo and ignored by git.

## ⚙️ Configuration Cheatsheet

| Setting | Location | Notes |
| --- | --- | --- |
| LLM provider | Settings panel → LLM | Choose between Ollama, OpenClaw proxy, OpenRouter, Groq. Empty inputs fall back to placeholders only. |
| Custom VRM | VRM upload button (camera panel) | Files stored under `%APPDATA%/local-companion/custom_vrms`; filename persists across sessions. |
| Vision assist | Settings → Visual Model | When enabled, screen captures are routed through `screen_vision_service.py` (auto-managed by Tauri). |


## 🧱 Architecture Snapshot

- **Frontend (Svelte)**: `VRMRenderer.svelte` hosts chat bubbles, microphone controls, viseme animation, and VRM file management.
- **Backend (Rust/Tauri)**: `src-tauri/src/lib.rs` exposes commands to persist settings, proxy HTTP calls to OpenClaw/Groq/OpenRouter/Ollama, and spin up helper services (vision/STT bootstrap).
- **Speech pipeline**: MediaRecorder → Deepgram API → selected LLM → ElevenLabs streaming chunker → WebAudio playback + viseme queue.
- **Persistence**: Settings JSON under `%APPDATA%/OllamaGUI` plus optional VRM binaries inside `%APPDATA%/local-companion/custom_vrms`.

## 🧪 Testing & Debugging Tips

- `npm run tauri dev` prints both frontend and Rust logs in one console. Search for tags like `[S2S]`, `[STT][Deepgram]`, or `[Chat]` to trace the speech pipeline.
- To reset the app to a pristine state, delete `%APPDATA%/OllamaGUI/settings.json` and restart. Default model placeholders (e.g., `mistral`, `http://127.0.0.1:18789`) will then show as empty fields awaiting user input.
- VRM issues? Clear `%APPDATA%/local-companion/custom_vrms` or use the new upload panel to overwrite the stored file.

## 📦 Building a Release

```bash
cd local-companion
npm run tauri build
```
This produces a signed Windows installer/exe in `src-tauri/target/release/`. Remember to bundle your own VRM/model assets or provide download links—none are included in the repo beyond lightweight defaults.

## 📝 License

Pending — choose the license that best matches your distribution plans before publishing. A permissive option (MIT/Apache-2.0) is typical for companion tools, but feel free to adapt.
>>>>>>> 6839fe2 (Initial commit)

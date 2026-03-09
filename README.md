<div align="center">

# ✨ V-Lucent
### **The Transparent, Screen-Aware 3D AI Companion**

[![Tauri](https://img.shields.io/badge/Tauri-2.0-FFC131?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5.0-FF3E00?style=for-the-badge&logo=svelte&logoColor=white)](https://svelte.dev/)
[![Rust](https://img.shields.io/badge/Rust-Stable-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue?style=for-the-badge)](LICENSE)

**V-Lucent** is a desktop-ready AI companion that blends a VRM avatar, speech-to-speech chat, and OpenClaw’s VPS brain into a single Tauri application. Windows builds are ready today (macOS/Linux later) with a slim Svelte front-end and Rust backend.

[Explore Docs](#-quick-start) • [Report Bug](https://github.com/codacipher/v-lucent/issues) • [Request Feature](https://github.com/codacipher/v-lucent/issues)

</div>

---

## 🚀 Highlights

- **🪟 Transparent Overlay** – Native desktop companion that floats with zero background chrome.
- **👁️ Screen Awareness** – Optional `screen_vision_service.py` loop summarizes your live screen.
- **🗣️ Speech-to-Speech** – MediaRecorder ➜ Deepgram (STT) ➜ multi-provider LLM ➜ ElevenLabs (TTS) with viseme sync.
- **🎭 3D Emotional Presence** – Three.js + @pixiv/three-vrm avatar with emotion-driven blendshapes and custom VRM upload.
- **🧠 Brain Switcher** – Hot-swap between Ollama, OpenClaw, OpenRouter, and Groq at runtime.
- **🧳 Legacy Isolation** – Heavy Docker/proxy experiments live under `legacy/` so the active app stays lightweight.

---

## 📁 Repository Layout

```
V-Lucent/
├── local-companion/         # Active Tauri + Svelte 5 application
│   ├── src/                 # Frontend (VRMRenderer, chat UI, settings)
│   ├── src-tauri/           # Rust backend (commands, proxy, vision services)
│   └── public/models/       # Default VRM assets
├── legacy/                  # Archived Docker/proxy experiments (ignored by default build)
└── README.md
```

---

## 🛠️ Quick Start

### Prerequisites

- Node.js 20+
- Rust toolchain (stable) + `cargo`
- Windows 10/11 with WebView2 runtime (see [Tauri checklist](https://tauri.app/v1/guides/getting-started/prerequisites/))

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

> **💡 Tip:** The Svelte UI runs at `localhost:1420`, while the Rust backend exposes helper APIs on `localhost:3030`.

---

## ⚙️ Configuration Cheatsheet

| Component | Location | Details |
|-----------|----------|---------|
| **LLM Provider** | Settings → LLM | Switch between Ollama, OpenClaw, OpenRouter, Groq |
| **Custom VRM** | Camera Panel | Drag & drop `.vrm`; stored under `%APPDATA%/local-companion` |
| **Audio Keys** | Settings → Audio | Add Deepgram (STT) + ElevenLabs (TTS) API keys |
| **Vision Assist** | Settings → Visual | Toggle `screen_vision_service.py` for context capture |

---

## 🧱 Architecture Snapshot

- **Frontend (Svelte 5):** `VRMRenderer.svelte` drives the Three.js scene, visemes, and chat UI.
- **Backend (Rust/Tauri):** `src-tauri/src/lib.rs` handles LLM proxying, native screen capture, settings persistence, and helper daemons.
- **Speech Pipeline:** MediaRecorder ➜ Deepgram ➜ chosen LLM ➜ ElevenLabs chunked playback for natural voice loops.
- **Persistence:** `%APPDATA%/OllamaGUI/settings.json` for secrets + `%APPDATA%/local-companion/custom_vrms` for avatars (both git-ignored).

---

## 🧪 Testing & Debugging Tips

- `npm run tauri dev` streams both frontend and Rust logs—search for tags like `[S2S]`, `[STT][Deepgram]`, `[Chat]`.
- Reset to defaults by deleting `%APPDATA%/OllamaGUI/settings.json` and relaunching.
- VRM acting up? Clear `%APPDATA%/local-companion/custom_vrms` or upload a new file.

---

## 📦 Building a Release

```bash
cd local-companion
npm run tauri build
```

Build artifacts land in `local-companion/src-tauri/target/release/`. Bundle your own VRM/model assets or ship an installer/downloader for large binaries.

---

## 🗺️ Roadmap

- **v1.0** – Tauri 2 / Svelte 5 core engine
- **v1.1** – Real-time viseme & lip-sync polish
- **v1.2** – Multi-provider routing + proactive vision loop
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


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

## 📁 Repository Layout

```
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

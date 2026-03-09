<script lang="ts">
  import { onMount } from "svelte";

  export let isOpen: boolean = false;
  export let invokeFunc: any = null;

  interface AppSettings {
    api_type: string;
    ollama_endpoint: string;
    ollama_model: string;
    openclaw_endpoint: string;
    openrouter_api_key: string;
    openrouter_provider: string;
    groq_api_key: string;
    groq_model: string;
    tts_engine: string;
    tts_language: string;
    tts_api_key: string;
    tts_voice_id: string;
    tts_ref_name: string;
    tts_ref_path: string;
    vision_api_type?: string;
    vision_model?: string;
    vision_api_key?: string;
    use_vision_model?: boolean;
  }

  let settings: AppSettings = {
    api_type: "ollama",
    ollama_endpoint: "",
    ollama_model: "",
    openclaw_endpoint: "",
    openrouter_api_key: "",
    openrouter_provider: "",
    groq_api_key: "",
    groq_model: "",
    tts_engine: "xtts_v2",
    tts_language: "tr",
    tts_api_key: "",
    tts_voice_id: "",
    tts_ref_name: "",
    tts_ref_path: "",
    vision_api_type: "",
    vision_model: "",
    vision_api_key: "",
    use_vision_model: true,
  };

  let availableModels: string[] = [];
  let loadingModels: boolean = false;
  let saveStatus: string = "";
  let refFileInput: HTMLInputElement | null = null;

  const refLabel = () => {
    if (settings.tts_ref_name?.trim()) return settings.tts_ref_name;
    if (settings.tts_ref_path?.trim()) {
      const parts = settings.tts_ref_path.split(/[/\\]/);
      return parts[parts.length - 1] || settings.tts_ref_path;
    }
    return "";
  };

  const uploadRefFile = async (file: File) => {
    const form = new FormData();
    form.append("file", file);
    // Optimistically show selected file name
    settings = { ...settings, tts_ref_name: file.name };
    try {
      const res = await fetch("http://127.0.0.1:5000/upload-ref", { method: "POST", body: form });
      if (res.ok) {
        const data = await res.json();
        if (data?.saved_path) {
          settings = {
            ...settings,
            tts_ref_name: file.name,
            tts_ref_path: data.saved_path,
          };
          await saveSettings();
          console.log("[TTS] ref uploaded to", data.saved_path);
        }
      } else {
        console.warn("[TTS] upload failed", res.status);
      }
    } catch (e) {
      console.error("[TTS] upload error:", e);
    }
  };


  const loadSettings = async () => {
    try {
      if (!invokeFunc) {
        console.warn("[Settings] invokeFunc not available");
        return;
      }
      const loaded = await invokeFunc("get_settings");
      const merged = { ...settings, ...loaded };
      if (!merged.tts_ref_name?.trim() && merged.tts_ref_path?.trim()) {
        merged.tts_ref_name = merged.tts_ref_path.split(/[/\\]/).pop() || "";
      }
      settings = merged;
      console.log("[Settings] Loaded:", settings);
    } catch (error) {
      console.error("[Settings] Error loading:", error);
    }
  };

  const saveSettings = async () => {
    try {
      if (!invokeFunc) {
        console.error("[Settings] invokeFunc not available");
        saveStatus = "✗ Error: No Tauri API";
        return;
      }
      const saved = await invokeFunc("update_settings", { settings });
      settings = {
        ...settings,
        ...saved,
        tts_ref_name:
          (saved?.tts_ref_name ?? "").toString().trim() === ""
            ? settings.tts_ref_name
            : saved.tts_ref_name,
        tts_ref_path:
          (saved?.tts_ref_path ?? "").toString().trim() === ""
            ? settings.tts_ref_path
            : saved.tts_ref_path,
      };
      saveStatus = "✓ Saved";
      setTimeout(() => (saveStatus = ""), 2000);
      console.log("[Settings] Saved:", settings);
    } catch (error) {
      console.error("[Settings] Error saving:", error);
      saveStatus = "✗ Error";
    }
  };

  const fetchModels = async () => {
    if (!settings.ollama_endpoint) {
      console.warn("[Settings] Endpoint is empty");
      return;
    }

    if (!invokeFunc) {
      console.error("[Settings] invokeFunc not available");
      return;
    }

    loadingModels = true;
    try {
      const models = await invokeFunc("get_ollama_models", {
        endpoint: settings.ollama_endpoint,
      });
      availableModels = models;
      console.log("[Settings] Models fetched:", models);
    } catch (error) {
      console.error("[Settings] Error fetching models:", error);
      availableModels = [];
    } finally {
      loadingModels = false;
    }
  };

  const handleApiTypeChange = () => {
    if (settings.api_type === "ollama") {
      fetchModels();
    }
  };

  const handleEndpointChange = () => {
    if (settings.api_type === "ollama") {
      availableModels = [];
    }
  };

  const closePanel = () => {
    isOpen = false;
  };

  onMount(() => {
    loadSettings();
  });
</script>

{#if isOpen}
  <!-- NOTE: Bu panel şu an kullanılmıyor; VRMRenderer içindeki ayarlar geçerlidir. -->
  <div class="settings-panel" class:visible={isOpen}>
    <div class="settings-header">
      <h3>⚙️ Settings</h3>
      <button class="settings-close-btn" on:click={closePanel}>✕</button>
    </div>

    <div class="settings-content">
      <div class="section-title">LLM</div>
      <div class="settings-group">
        <label for="api-type">API Type</label>
        <select id="api-type" bind:value={settings.api_type} on:change={handleApiTypeChange}>
          <option value="ollama">Ollama</option>
          <option value="openclaw">OpenClaw</option>
          <option value="openrouter">OpenRouter</option>
          <option value="groq">Groq</option>
        </select>
      </div>

      {#if settings.api_type === "ollama"}
        <div class="settings-group">
          <label for="ollama-endpoint">Ollama Endpoint</label>
          <input
            id="ollama-endpoint"
            type="text"
            bind:value={settings.ollama_endpoint}
            on:change={handleEndpointChange}
            placeholder="http://127.0.0.1:11434"
          />
          <button class="fetch-models-btn" on:click={() => fetchModels()} disabled={loadingModels}>
            {loadingModels ? "Loading..." : "Fetch Models"}
          </button>
        </div>

        <div class="settings-group">
          <label for="ollama-model">Model</label>
          {#if availableModels.length > 0}
            <select id="ollama-model" bind:value={settings.ollama_model}>
              {#each availableModels as model}
                <option value={model}>{model}</option>
              {/each}
            </select>
          {:else}
            <input
              id="ollama-model"
              type="text"
              bind:value={settings.ollama_model}
              placeholder="mistral"
            />
          {/if}
        </div>
      {:else if settings.api_type === "openclaw"}
        <div class="settings-group">
          <label for="openclaw-endpoint">OpenClaw Endpoint</label>
          <input
            id="openclaw-endpoint"
            type="text"
            bind:value={settings.openclaw_endpoint}
            placeholder="http://127.0.0.1:18789"
          />
        </div>
      {:else if settings.api_type === "openrouter"}
        <div class="settings-group">
          <label for="openrouter-api-key">OpenRouter API Key</label>
          <input
            id="openrouter-api-key"
            type="password"
            bind:value={settings.openrouter_api_key}
            placeholder="sk-or-..."
          />
        </div>
        <div class="settings-group">
          <label for="openrouter-provider">Provider / Model</label>
          <input
            id="openrouter-provider"
            type="text"
            bind:value={settings.openrouter_provider}
            placeholder="google/gemini-3-flash-preview"
          />
        </div>
      {:else if settings.api_type === "groq"}
        <div class="settings-group">
          <label for="groq-api-key">Groq API Key</label>
          <input
            id="groq-api-key"
            type="password"
            bind:value={settings.groq_api_key}
            placeholder="gsk_********"
          />
        </div>
        <div class="settings-group">
          <label for="groq-model">Groq Model</label>
          <input
            id="groq-model"
            type="text"
            bind:value={settings.groq_model}
            placeholder="llama-3.3-70b-versatile"
          />
        </div>
      {/if}

      <div class="section-separator"></div>
      <div class="section-title">Visual Model</div>
      <p class="hint">Boş bırakılırsa LLM ayarları kullanılır.</p>
      <div class="settings-group inline">
        <label class="checkbox-row">
          <input
            type="checkbox"
            bind:checked={settings.use_vision_model}
            on:change={() => saveSettings()}
          />
          <span>Use visual model (send screen captures)</span>
          <span class="tooltip" title="Sends the current screen as an image to the vision model before each user message to enrich context.">?</span>
        </label>
      </div>
      <div class="settings-group">
        <label for="vision-api-type">API Type</label>
        <select id="vision-api-type" bind:value={settings.vision_api_type}>
          <option value="">(LLM ile aynı)</option>
          <option value="ollama">Ollama</option>
          <option value="openrouter">OpenRouter</option>
          <option value="groq">Groq</option>
          <option value="openai">OpenAI</option>
        </select>
      </div>
      <div class="settings-group">
        <label for="vision-model">Model</label>
        <input
          id="vision-model"
          type="text"
          bind:value={settings.vision_model}
          placeholder="meta-llama/llama-3.2-11b-vision-instruct"
        />
      </div>
      <div class="settings-group">
        <label for="vision-api-key">API Key (opsiyonel)</label>
        <input
          id="vision-api-key"
          type="password"
          bind:value={settings.vision_api_key}
          placeholder="Boşsa LLM key kullanılır"
        />
      </div>

      <div class="section-separator"></div>
      <div class="section-title">TTS</div>
      <div class="settings-group">
        <label for="tts-api-key">ElevenLabs API Key</label>
        <input
          id="tts-api-key"
          type="password"
          bind:value={settings.tts_api_key}
          placeholder="Enter ElevenLabs API key"
        />
      </div>
      <div class="settings-group">
        <label for="tts-voice-id">ElevenLabs Voice ID</label>
        <input
          id="tts-voice-id"
          type="text"
          bind:value={settings.tts_voice_id}
          placeholder="Voice ID (leave blank to use default)"
        />
      </div>

      <div class="settings-actions">
        <button class="save-btn" on:click={() => saveSettings()}>Save Settings</button>
        {#if saveStatus}
          <span class="save-status">{saveStatus}</span>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .settings-panel {
    position: absolute;
    bottom: 80px;
    right: 20px;
    width: 320px;
    background: rgba(20, 20, 30, 0.95);
    border: 1px solid rgba(100, 150, 255, 0.3);
    border-radius: 12px;
    z-index: 50;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    backdrop-filter: blur(10px);
    animation: slideUp 0.3s ease-out;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .settings-header {
    padding: 12px 16px;
    border-bottom: 1px solid rgba(100, 150, 255, 0.2);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .settings-header h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: #6495ed;
  }

  .settings-close-btn {
    background: none;
    border: none;
    color: #a0c8ff;
    font-size: 18px;
    cursor: pointer;
    padding: 4px 8px;
    transition: all 0.2s;
  }

  .settings-close-btn:hover {
    color: #6495ed;
    transform: scale(1.2);
  }

  .settings-content {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 400px;
    overflow-y: auto;
  }

  .settings-content::-webkit-scrollbar {
    width: 4px;
  }

  .settings-content::-webkit-scrollbar-track {
    background: transparent;
  }

  .settings-content::-webkit-scrollbar-thumb {
    background: rgba(100, 150, 255, 0.3);
    border-radius: 2px;
  }

  .settings-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .section-title {
    font-size: 12px;
    font-weight: 700;
    color: #a0c8ff;
    text-transform: uppercase;
    letter-spacing: 0.6px;
    margin-top: 4px;
  }

  .section-separator {
    height: 1px;
    background: rgba(100, 150, 255, 0.2);
    margin: 6px 0;
  }

  .settings-group label {
    font-size: 12px;
    font-weight: 600;
    color: #8ab4f8;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .dropzone {
    border: 1px dashed rgba(100, 150, 255, 0.4);
    border-radius: 8px;
    padding: 12px;
    background: rgba(30, 30, 50, 0.4);
    color: #a0c8ff;
    cursor: pointer;
    position: relative;
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  .dropzone:hover {
    border-color: #6495ed;
    background: rgba(40, 50, 80, 0.6);
  }

  .dropzone input[type="file"] {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }

  .dropzone-text {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .ref-path {
    word-break: break-all;
    color: #cfe0ff;
    font-weight: 600;
  }

  .hint {
    margin: 4px 0 0;
    font-size: 11px;
    color: rgba(160, 200, 255, 0.7);
  }

  .settings-group input,
  .settings-group select {
    padding: 8px 12px;
    background: rgba(30, 30, 50, 0.8);
    border: 1px solid rgba(100, 150, 255, 0.3);
    border-radius: 6px;
    color: #a0c8ff;
    font-size: 12px;
    outline: none;
    transition: all 0.2s;
  }

  .settings-group input:focus,
  .settings-group select:focus {
    border-color: #6495ed;
    box-shadow: 0 0 8px rgba(100, 150, 255, 0.2);
  }

  .settings-group input::placeholder {
    color: rgba(160, 200, 255, 0.5);
  }

  .fetch-models-btn {
    padding: 6px 12px;
    background: rgba(100, 150, 255, 0.2);
    border: 1px solid rgba(100, 150, 255, 0.4);
    border-radius: 6px;
    color: #a0c8ff;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .fetch-models-btn:hover:not(:disabled) {
    background: rgba(100, 150, 255, 0.3);
    border-color: #6495ed;
  }

  .fetch-models-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .settings-actions {
    display: flex;
    gap: 8px;
    align-items: center;
    margin-top: 8px;
  }

  .save-btn {
    flex: 1;
    padding: 8px 16px;
    background: linear-gradient(135deg, #6495ed, #4169e1);
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .save-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(100, 150, 255, 0.3);
  }

  .save-btn:active {
    transform: translateY(0);
  }

  .save-status {
    font-size: 11px;
    color: #8ab4f8;
    font-weight: 600;
  }
</style>

<script lang="ts">
  import { WSClient, type ConnectionStatus, type ChatMessage } from "./lib/ws";
  import { onMount, onDestroy } from "svelte";
  import VRMRenderer from "./lib/VRMRenderer.svelte";

  let status: ConnectionStatus = $state("disconnected");
  let lastEmotion: string = $state("neutral");
  let lastText: string = $state("");
  let showBubble: boolean = $state(false);
  let bubbleTimer: ReturnType<typeof setTimeout> | null = null;
  let vrmRenderer: VRMRenderer | null = null;

  const WS_URL = import.meta.env.VITE_WS_URL?.trim() || "";
  const client = WS_URL
    ? new WSClient({
        url: WS_URL,
        onStatusChange: (s) => {
          status = s;
        },
        onMessage: (msg) => {
          if (msg.channel === "chat") {
            const chat = msg as ChatMessage;
            lastEmotion = chat.payload.emotion;
            lastText = chat.payload.text;
            showBubble = true;

            if (bubbleTimer) clearTimeout(bubbleTimer);
            bubbleTimer = setTimeout(() => {
              showBubble = false;
            }, 8000);
          }
        },
      })
    : null;

  onMount(() => {
    if (client) {
      client.connect();
    } else {
      status = "disconnected";
    }
  });
  onDestroy(() => {
    client?.disconnect();
  });
</script>

<main>
  {#if showBubble && lastText}
    <div class="speech-bubble">
      <span class="emotion-tag">{lastEmotion}</span>
      <p class="bubble-text">{lastText}</p>
    </div>
  {/if}

  <div class="status-dot" class:connected={status === "connected"} class:connecting={status === "connecting"}
    title={status}></div>

  <div class="avatar-container">
    <VRMRenderer bind:this={vrmRenderer} emotion={lastEmotion} modelPath="/models/model1.vrm" />
  </div>
</main>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent;
  }

  :global(html) {
    background: transparent;
  }

  main {
    position: relative;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    width: 100vw;
    height: 100vh;
    color: white;
    font-family: system-ui, sans-serif;
    cursor: grab;
    user-select: none;
    -webkit-user-select: none;
  }

  main:active {
    cursor: grabbing;
  }

  .avatar-container {
    width: 100%;
    height: 100%;
    position: absolute;
    top: 0;
    left: 0;
  }

  .speech-bubble {
    position: absolute;
    top: 12px;
    left: 50%;
    transform: translateX(-50%);
    background: rgba(30, 20, 50, 0.9);
    border: 1px solid rgba(108, 92, 231, 0.5);
    border-radius: 12px;
    padding: 10px 14px;
    max-width: 320px;
    pointer-events: none;
    animation: fadeIn 0.3s ease;
  }

  .emotion-tag {
    display: inline-block;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: #a29bfe;
    background: rgba(108, 92, 231, 0.2);
    padding: 2px 8px;
    border-radius: 6px;
    margin-bottom: 4px;
  }

  .bubble-text {
    margin: 6px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #e0e0e0;
  }

  .status-dot {
    position: absolute;
    top: 8px;
    right: 8px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #636e72;
    pointer-events: none;
  }

  .status-dot.connected {
    background: #00b894;
    box-shadow: 0 0 6px #00b894;
  }

  .status-dot.connecting {
    background: #fdcb6e;
    animation: pulse 1s infinite;
  }

  @keyframes fadeIn {
    from { opacity: 0; transform: translateX(-50%) translateY(-8px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
</style>

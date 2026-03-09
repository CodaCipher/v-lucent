<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import init, { CompanionPuppet, init_panic_hook } from "./wasm-pkg/companion_wasm.js";
  import { EmotionController } from "./emotions";

  let canvas: HTMLCanvasElement;
  let puppet: CompanionPuppet | null = null;
  let animFrameId: number = 0;
  let paramNames: string[] = [];
  let startTime: number = 0;
  let lastFrameTime: number = 0;
  let nextBlinkTime: number = 0;
  let blinkPhase: number = -1;
  let errorMessage: string | null = null;
  let cameraScale: number = 0.1;

  const emotionCtrl = new EmotionController(3.0);
  let currentEmotion: string = "neutral";

  export let modelUrl: string = "/models/aka.inx";
  export let emotion: string = "neutral";

  $: if (emotion !== currentEmotion) {
    currentEmotion = emotion;
    emotionCtrl.setEmotion(emotion);
  }

  function scheduleNextBlink() {
    nextBlinkTime = performance.now() + 2000 + Math.random() * 5000;
  }

  function safeSet(name: string, x: number, y: number = 0) {
    try { puppet!.set_param(name, x, y); } catch {}
  }

  function handleWheel(e: WheelEvent) {
    if (!puppet) return;
    try {
      e.preventDefault();
      const zoomSpeed = 0.05;
      const direction = e.deltaY > 0 ? 1 : -1;
      const newScale = Math.max(0.01, Math.min(1.0, cameraScale + direction * zoomSpeed));
      puppet.set_camera_scale(newScale);
      cameraScale = newScale;
      console.log(`[Zoom] Scale: ${newScale.toFixed(3)}`);
    } catch (err) {
      console.error("Wheel handler error:", err);
      errorMessage = `Zoom error: ${err}`;
    }
  }

  function wheelAction(node: HTMLElement) {
    const handler = (e: WheelEvent) => handleWheel(e);
    node.addEventListener("wheel", handler, { passive: false });
    return {
      destroy() {
        node.removeEventListener("wheel", handler);
      }
    };
  }

  function applyAnimations(timestamp: number) {
    if (!puppet) return;
    const t = (timestamp - startTime) / 1000;
    const dt = lastFrameTime === 0 ? 0.016 : (timestamp - lastFrameTime) / 1000;
    lastFrameTime = timestamp;

    // 1. Emotion params (smooth lerp) - DISABLED
    // const emotionParams = emotionCtrl.update(dt);

    // 2. Idle: breathing
    safeSet("Breath", Math.sin(t * 1.5) * 0.5 + 0.5, 0);

    // 3. Idle: head sway (Emotion offsets removed)
    safeSet(
      "Head:: Yaw-Pitch",
      Math.sin(t * 0.4) * 0.15,
      Math.sin(t * 0.3) * 0.08
    );

    // 4. Idle: body sway (Emotion offsets removed)
    safeSet(
      "Body:: Yaw-Pitch",
      Math.sin(t * 0.25) * 0.05,
      Math.sin(t * 0.2) * 0.03
    );

    /*
    // 5. Emotion: mouth
    const mouthShape = emotionParams["Mouth:: Shape"] ?? [0, 0];
    const mouthWidth = emotionParams["Mouth:: Width"] ?? [0, 0];
    safeSet("Mouth:: Shape", mouthShape[0], mouthShape[1]);
    safeSet("Mouth:: Width", mouthWidth[0], mouthWidth[1]);

    // 6. Emotion: body roll
    const bodyRoll = emotionParams["Body:: Roll"] ?? [0, 0];
    safeSet("Body:: Roll", bodyRoll[0], bodyRoll[1]);
    */

    // 7. Eye blinking (Emotion offsets removed)
    if (timestamp >= nextBlinkTime && blinkPhase < 0) {
      blinkPhase = 0;
    }

    let blinkVal = 0;
    if (blinkPhase >= 0) {
      blinkPhase += 0.15;
      blinkVal = blinkPhase <= 1 ? blinkPhase : Math.max(0, 2 - blinkPhase);
      if (blinkPhase >= 2) {
        blinkPhase = -1;
        blinkVal = 0;
        scheduleNextBlink();
      }
    }

    safeSet("Eye:: Left:: Blink", blinkVal, 0);
    safeSet("Eye:: Right:: Blink", blinkVal, 0);
  }

  async function initPuppet() {
    try {
      await init();
      init_panic_hook();

      console.log(`[Puppet] Loading model from ${modelUrl}...`);
      const res = await fetch(modelUrl);
      if (!res.ok) throw new Error(`Failed to fetch model: ${res.statusText}`);
      
      const buffer = await res.arrayBuffer();
      if (buffer.byteLength === 0) throw new Error("Model file is empty");
      
      const bytes = new Uint8Array(buffer);
      console.log(`[Puppet] Model fetched, size: ${bytes.length} bytes`);

      const rect = canvas.parentElement!.getBoundingClientRect();
      canvas.width = rect.width * window.devicePixelRatio;
      canvas.height = rect.height * window.devicePixelRatio;
      canvas.style.width = rect.width + "px";
      canvas.style.height = rect.height + "px";

      console.log("[Puppet] Initializing WASM puppet...");
      puppet = new CompanionPuppet("puppet-canvas", bytes);
      console.log("[Puppet] WASM puppet initialized successfully");
      
      puppet.resize(canvas.width, canvas.height);
      puppet.set_camera_scale(0.1); // User requested to "move back" (zoom out)

      try {
        const namesJson = puppet.get_param_names();
        paramNames = JSON.parse(namesJson);
        console.log("[Puppet] Available params:", paramNames);
      } catch (e) {
        console.warn("[Puppet] Could not get param names:", e);
      }

      startTime = performance.now();
      scheduleNextBlink();
      emotionCtrl.setEmotion("neutral");

      function renderLoop(timestamp: number) {
        if (puppet) {
          puppet.begin_frame(timestamp);
          applyAnimations(timestamp);
          puppet.end_and_draw();
        }
        animFrameId = requestAnimationFrame(renderLoop);
      }
      animFrameId = requestAnimationFrame(renderLoop);
    } catch (e: any) {
      console.error("[Puppet] Init failed:", e);
      errorMessage = e.toString();
    }
  }

  onMount(() => {
    initPuppet();
  });

  onDestroy(() => {
    if (animFrameId) cancelAnimationFrame(animFrameId);
    if (puppet) puppet.free();
  });

  export function setParam(name: string, x: number, y: number = 0) {
    if (puppet) {
      try { puppet.set_param(name, x, y); } catch {}
    }
  }

  export function getParamNames(): string[] {
    return paramNames;
  }
</script>

{#if errorMessage}
  <div class="error-overlay">
    <p>Puppet Error: {errorMessage}</p>
  </div>
{/if}

<canvas id="puppet-canvas" bind:this={canvas} use:wheelAction></canvas>

<style>
  canvas {
    width: 100%;
    height: 100%;
    display: block;
    background: transparent;
  }

  .error-overlay {
    position: absolute;
    top: 10px;
    left: 10px;
    background: rgba(255, 0, 0, 0.8);
    color: white;
    padding: 10px;
    border-radius: 4px;
    z-index: 1000;
    max-width: 80%;
  }
</style>

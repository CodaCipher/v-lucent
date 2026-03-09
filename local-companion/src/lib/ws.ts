export type ConnectionStatus = "disconnected" | "connecting" | "connected";

export interface ChatMessage {
  channel: "chat";
  payload: {
    text: string;
    emotion: string;
    intensity: number;
    tts_audio?: string;
    viseme_timeline?: unknown[];
  };
}

export interface MediaMessage {
  channel: "media";
  payload: {
    type: "image" | "video" | "stream";
    display: "hologram" | "background" | "pip";
    mime: string;
    data?: string;
    stream_id?: string;
    chunk_index?: number;
    total_chunks?: number;
    metadata?: {
      duration_ms?: number;
      width?: number;
      height?: number;
    };
  };
}

export type CompanionMessage = ChatMessage | MediaMessage;

export interface WSClientOptions {
  url: string;
  reconnectInterval?: number;
  maxReconnectInterval?: number;
  onMessage?: (msg: CompanionMessage) => void;
  onStatusChange?: (status: ConnectionStatus) => void;
}

export class WSClient {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectInterval: number;
  private maxReconnectInterval: number;
  private currentReconnectInterval: number;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private intentionalClose = false;
  private onMessage: (msg: CompanionMessage) => void;
  private onStatusChange: (status: ConnectionStatus) => void;

  constructor(options: WSClientOptions) {
    this.url = options.url;
    this.reconnectInterval = options.reconnectInterval ?? 1000;
    this.maxReconnectInterval = options.maxReconnectInterval ?? 30000;
    this.currentReconnectInterval = this.reconnectInterval;
    this.onMessage = options.onMessage ?? (() => {});
    this.onStatusChange = options.onStatusChange ?? (() => {});
  }

  connect(): void {
    if (this.ws) return;

    this.intentionalClose = false;
    this.onStatusChange("connecting");

    try {
      this.ws = new WebSocket(this.url);
    } catch {
      this.onStatusChange("disconnected");
      this.scheduleReconnect();
      return;
    }

    this.ws.onopen = () => {
      this.currentReconnectInterval = this.reconnectInterval;
      this.onStatusChange("connected");
    };

    this.ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data) as CompanionMessage;
        this.onMessage(msg);
      } catch {
        console.warn("[WS] Failed to parse message:", event.data);
      }
    };

    this.ws.onclose = () => {
      this.ws = null;
      this.onStatusChange("disconnected");
      if (!this.intentionalClose) {
        this.scheduleReconnect();
      }
    };

    this.ws.onerror = () => {
      this.ws?.close();
    };
  }

  disconnect(): void {
    this.intentionalClose = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.ws?.close();
    this.ws = null;
    this.onStatusChange("disconnected");
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, this.currentReconnectInterval);

    this.currentReconnectInterval = Math.min(
      this.currentReconnectInterval * 1.5,
      this.maxReconnectInterval
    );
  }

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}

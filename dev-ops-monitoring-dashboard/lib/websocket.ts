/**
 * WebSocket Manager for real-time updates
 * Handles connections to server WebSocket endpoints with automatic reconnection
 */

export type WsMetricMessage = {
  Metric: {
    agent_id: string;
    metric_name: string;
    value: number;
    timestamp: string;
  };
};

export type WsLogMessage = {
  Log: {
    agent_id: string;
    level: string;
    message: string;
    timestamp: string;
  };
};

export type WsAlertMessage = {
  Alert: {
    alert_id: number;
    agent_id: string;
    severity: string;
    message: string;
    state: string;
  };
};

export type WsHeartbeat = "Heartbeat";

export type WsMessage =
  | WsMetricMessage
  | WsLogMessage
  | WsAlertMessage
  | WsHeartbeat;

export type WebSocketCallback<T = WsMessage> = (message: T) => void;

export interface WebSocketManagerOptions {
  url: string;
  reconnectDelay?: number;
  maxReconnectDelay?: number;
  reconnectAttempts?: number;
  debug?: boolean;
}

export class WebSocketManager {
  private ws: WebSocket | null = null;
  private url: string;
  private callbacks: Set<WebSocketCallback> = new Set();
  private reconnectDelay: number;
  private maxReconnectDelay: number;
  private reconnectAttempts: number;
  private currentAttempt: number = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private shouldReconnect: boolean = true;
  private debug: boolean;
  private connectionState: "connecting" | "connected" | "disconnected" =
    "disconnected";
  private stateChangeCallbacks: Set<(state: string) => void> = new Set();

  constructor(options: WebSocketManagerOptions) {
    this.url = options.url;
    this.reconnectDelay = options.reconnectDelay || 1000;
    this.maxReconnectDelay = options.maxReconnectDelay || 30000;
    this.reconnectAttempts = options.reconnectAttempts || Infinity;
    this.debug = options.debug || false;
  }

  private log(...args: any[]) {
    if (this.debug) {
      console.log("[WebSocket]", ...args);
    }
  }

  private error(...args: any[]) {
    console.error("[WebSocket]", ...args);
  }

  private setState(state: "connecting" | "connected" | "disconnected") {
    if (this.connectionState !== state) {
      this.connectionState = state;
      this.log("State changed to:", state);
      this.stateChangeCallbacks.forEach((cb) => cb(state));
    }
  }

  public getState() {
    return this.connectionState;
  }

  public onStateChange(callback: (state: string) => void) {
    this.stateChangeCallbacks.add(callback);
    return () => this.stateChangeCallbacks.delete(callback);
  }

  public connect() {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.log("Already connected");
      return;
    }

    if (this.ws?.readyState === WebSocket.CONNECTING) {
      this.log("Connection in progress");
      return;
    }

    this.log("Connecting to", this.url);
    this.setState("connecting");

    try {
      this.ws = new WebSocket(this.url);

      this.ws.onopen = () => {
        this.log("Connected successfully");
        this.setState("connected");
        this.currentAttempt = 0;
        this.reconnectDelay = 1000;
      };

      this.ws.onmessage = (event) => {
        try {
          const message: WsMessage = JSON.parse(event.data);

          // Ignore heartbeat messages
          if (message === "Heartbeat") {
            return;
          }

          this.log("Received message:", message);
          this.callbacks.forEach((cb) => cb(message));
        } catch (err) {
          this.error("Failed to parse message:", err);
        }
      };

      this.ws.onerror = (event) => {
        this.error("WebSocket error:", event);
      };

      this.ws.onclose = (event) => {
        this.log("Connection closed:", event.code, event.reason);
        this.setState("disconnected");
        this.ws = null;

        if (this.shouldReconnect) {
          this.scheduleReconnect();
        }
      };
    } catch (err) {
      this.error("Failed to create WebSocket:", err);
      this.setState("disconnected");
      if (this.shouldReconnect) {
        this.scheduleReconnect();
      }
    }
  }

  private scheduleReconnect() {
    if (this.currentAttempt >= this.reconnectAttempts) {
      this.error(
        "Max reconnection attempts reached:",
        this.reconnectAttempts
      );
      return;
    }

    this.currentAttempt++;
    const delay = Math.min(
      this.reconnectDelay * Math.pow(1.5, this.currentAttempt - 1),
      this.maxReconnectDelay
    );

    this.log(
      `Reconnecting in ${delay}ms (attempt ${this.currentAttempt}/${this.reconnectAttempts})`
    );

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }

  public subscribe(callback: WebSocketCallback) {
    this.callbacks.add(callback);
    return () => this.unsubscribe(callback);
  }

  public unsubscribe(callback: WebSocketCallback) {
    this.callbacks.delete(callback);
  }

  public disconnect() {
    this.log("Disconnecting...");
    this.shouldReconnect = false;

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }

    this.setState("disconnected");
  }

  public send(message: any) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      this.error("Cannot send message: WebSocket not connected");
    }
  }
}

/**
 * Create a WebSocket manager for a specific endpoint
 */
export function createWebSocketManager(
  endpoint: "metrics" | "logs" | "alerts",
  baseUrl?: string
): WebSocketManager {
  // Determine the WebSocket URL
  let wsUrl: string;
  
  if (baseUrl) {
    wsUrl = baseUrl.replace(/^http/, 'ws');
  } else if (typeof window !== 'undefined') {
    // In browser: use environment variable or derive from current location
    const envUrl = process.env.NEXT_PUBLIC_API_URL;
    if (envUrl && envUrl !== 'http://localhost:8080') {
      wsUrl = envUrl.replace(/^http/, 'ws');
    } else {
      // Fallback: assume API is on port 8080 of the same host
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.hostname;
      wsUrl = `${protocol}//${host}:8080`;
    }
  } else {
    // Server-side: use environment variable or default
    const envUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
    wsUrl = envUrl.replace(/^http/, 'ws');
  }
  
  const url = `${wsUrl}/api/v1/ws/${endpoint}`;
  
  console.log(`[WebSocket] Creating manager for ${endpoint} at ${url}`);

  return new WebSocketManager({
    url,
    reconnectDelay: 1000,
    maxReconnectDelay: 30000,
    reconnectAttempts: Infinity,
    debug: true, // Always enable debug for now
  });
}

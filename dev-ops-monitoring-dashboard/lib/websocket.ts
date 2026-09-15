import { getRealtimeToken } from './auth-utils';

/**
 * WebSocket Manager for real-time updates
 * Handles connections to server WebSocket endpoints with automatic reconnection
 */

export type WsMetricMessage = {
  type: "metric";
  agent_id: string;
  metric_name: string;
  value: number;
  timestamp: string;
};

export type WsMetricBatchMessage = {
  type: "metric_batch";
  agent_id: string;
  metrics: Record<string, number>;
  timestamp: string;
};

export type ProcessSnapshot = {
  pid: number;
  parent_pid: number | null;
  user: string;
  state: string;
  cpu_percent: number;
  memory_bytes: number;
  virtual_memory_bytes: number;
  disk_read_bytes: number;
  disk_written_bytes: number;
  run_time_seconds: number;
  command: string;
};

export type WsProcessSnapshotMessage = {
  type: "process_snapshot";
  agent_id: string;
  processes: ProcessSnapshot[];
  timestamp: string;
};

export type WsLogMessage = {
  type: "log";
  agent_id: string;
  level: string;
  message: string;
  source: string;
  timestamp: string;
};

export type WsAlertMessage = {
  type: "alert";
  alert_id: string;
  agent_id: string;
  severity: string;
  message: string;
  state: string;
};

export type WsAgentSnapshot = {
  id: string;
  name: string;
  status: string;
  last_seen: string;
};

export type WsMetricSnapshot = {
  agent_id: string;
  metric_name: string;
  latest_value: number;
  timestamp: string;
};

export type WsInitialStateMessage = {
  type: "initial_state";
  agents: WsAgentSnapshot[];
  metrics: WsMetricSnapshot[];
};

export type WsHistoricalMetricsMessage = {
  type: "historical_metrics";
  agent_id: string;
  metric_name: string;
  data_points: { timestamp: string; value: number }[];
};

export type WsHeartbeat = { type: "heartbeat" };

export type WsMessage =
  | WsMetricMessage
  | WsMetricBatchMessage
  | WsProcessSnapshotMessage
  | WsLogMessage
  | WsAlertMessage
  | WsInitialStateMessage
  | WsHistoricalMetricsMessage
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
        const token = getRealtimeToken();
        if (!token) {
          this.error("Cannot authenticate WebSocket: no active credential");
          this.shouldReconnect = false;
          this.ws?.close(1008, "Authentication required");
          return;
        }
        this.ws?.send(JSON.stringify({ type: "authenticate", token }));
        this.log("Connected successfully");
        this.setState("connected");
        this.currentAttempt = 0;
        this.reconnectDelay = 1000;
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);

          // Ignore heartbeat messages
          if (message.type === "heartbeat") {
            this.log("Received heartbeat");
            return;
          }

          this.log("Received message:", message.type, message);
          this.callbacks.forEach((cb) => cb(message as WsMessage));
        } catch (err) {
          this.error("Failed to parse message:", err, event.data);
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
  } else if (process.env.NEXT_PUBLIC_WS_URL) {
    wsUrl = process.env.NEXT_PUBLIC_WS_URL;
  } else if (typeof window !== 'undefined') {
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    wsUrl = `${wsProtocol}//${window.location.hostname}:8080`;
  } else {
    // Server-side: use environment variable or default
    wsUrl = 'ws://localhost:8080';
  }
  
  const url = `${wsUrl}/api/v1/ws/${endpoint}`;
  
  console.log(`[WebSocket] Creating manager for ${endpoint} at ${url}`);

  return new WebSocketManager({
    url,
    reconnectDelay: 1000,
    maxReconnectDelay: 30000,
    reconnectAttempts: Infinity,
    debug: process.env.NODE_ENV === 'development',
  });
}

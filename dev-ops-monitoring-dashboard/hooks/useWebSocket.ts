"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import {
  WebSocketManager,
  createWebSocketManager,
  WsMessage,
  WsMetricMessage,
  WsLogMessage,
  WsAlertMessage,
  WsInitialStateMessage,
  WsAgentSnapshot,
  WsMetricSnapshot,
} from "@/lib/websocket";

export interface MetricsWebSocketCallbacks {
  onMetric?: (message: WsMetricMessage) => void;
  onInitialState?: (agents: WsAgentSnapshot[], metrics: WsMetricSnapshot[]) => void;
}

/**
 * Hook for real-time metrics updates with initial state sync
 * This is the PRIMARY hook for all live metrics data - no HTTP fallback!
 */
export function useMetricsWebSocket(callbacks: MetricsWebSocketCallbacks | ((message: WsMetricMessage) => void)) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<"connecting" | "connected" | "disconnected">("disconnected");
  const [initialStateReceived, setInitialStateReceived] = useState(false);
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  
  // Normalize callbacks
  const normalizedCallbacks = typeof callbacks === 'function' 
    ? { onMetric: callbacks } 
    : callbacks;
  
  const callbacksRef = useRef(normalizedCallbacks);
  
  // Keep callback ref updated
  useEffect(() => {
    callbacksRef.current = normalizedCallbacks;
  }, [normalizedCallbacks]);

  useEffect(() => {
    console.log("[useMetricsWebSocket] Initializing WebSocket connection...");
    
    // Create WebSocket manager
    const manager = createWebSocketManager("metrics");
    wsManagerRef.current = manager;

    // Subscribe to state changes
    const unsubscribeState = manager.onStateChange((state) => {
      console.log("[useMetricsWebSocket] Connection state:", state);
      setConnectionState(state as "connecting" | "connected" | "disconnected");
      setIsConnected(state === "connected");
      
      // Reset initial state flag on disconnect
      if (state === "disconnected") {
        setInitialStateReceived(false);
      }
    });

    // Subscribe to messages
    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      // Handle initial state message
      if (message.type === "initial_state") {
        console.log("[useMetricsWebSocket] Received initial state:", 
          (message as WsInitialStateMessage).agents.length, "agents,",
          (message as WsInitialStateMessage).metrics.length, "metrics"
        );
        setInitialStateReceived(true);
        callbacksRef.current.onInitialState?.(
          (message as WsInitialStateMessage).agents,
          (message as WsInitialStateMessage).metrics
        );
        return;
      }
      
      // Handle real-time metric updates
      if (message.type === "metric") {
        console.log("[useMetricsWebSocket] Received metric:", 
          (message as WsMetricMessage).agent_id,
          (message as WsMetricMessage).metric_name,
          (message as WsMetricMessage).value
        );
        callbacksRef.current.onMetric?.(message as WsMetricMessage);
      }
    });

    // Connect
    manager.connect();

    // Cleanup on unmount
    return () => {
      console.log("[useMetricsWebSocket] Cleaning up WebSocket connection...");
      unsubscribeState();
      unsubscribeMessages();
      manager.disconnect();
      wsManagerRef.current = null;
    };
  }, []); // Only run once on mount

  return { isConnected, connectionState, initialStateReceived };
}

/**
 * Hook for real-time log updates
 */
export function useLogsWebSocket(onLog: (message: WsLogMessage) => void) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<"connecting" | "connected" | "disconnected">("disconnected");
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  const onLogRef = useRef(onLog);
  
  useEffect(() => {
    onLogRef.current = onLog;
  }, [onLog]);

  useEffect(() => {
    console.log("[useLogsWebSocket] Initializing WebSocket connection...");
    
    const manager = createWebSocketManager("logs");
    wsManagerRef.current = manager;

    const unsubscribeState = manager.onStateChange((state) => {
      console.log("[useLogsWebSocket] Connection state:", state);
      setConnectionState(state as "connecting" | "connected" | "disconnected");
      setIsConnected(state === "connected");
    });

    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      if (message.type === "log") {
        onLogRef.current(message as WsLogMessage);
      }
    });

    manager.connect();

    return () => {
      console.log("[useLogsWebSocket] Cleaning up WebSocket connection...");
      unsubscribeState();
      unsubscribeMessages();
      manager.disconnect();
      wsManagerRef.current = null;
    };
  }, []);

  return { isConnected, connectionState };
}

/**
 * Hook for real-time alert updates
 */
export function useAlertsWebSocket(onAlert: (message: WsAlertMessage) => void) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<"connecting" | "connected" | "disconnected">("disconnected");
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  const onAlertRef = useRef(onAlert);
  
  useEffect(() => {
    onAlertRef.current = onAlert;
  }, [onAlert]);

  useEffect(() => {
    console.log("[useAlertsWebSocket] Initializing WebSocket connection...");
    
    const manager = createWebSocketManager("alerts");
    wsManagerRef.current = manager;

    const unsubscribeState = manager.onStateChange((state) => {
      console.log("[useAlertsWebSocket] Connection state:", state);
      setConnectionState(state as "connecting" | "connected" | "disconnected");
      setIsConnected(state === "connected");
    });

    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      if (message.type === "alert") {
        onAlertRef.current(message as WsAlertMessage);
      }
    });

    manager.connect();

    return () => {
      console.log("[useAlertsWebSocket] Cleaning up WebSocket connection...");
      unsubscribeState();
      unsubscribeMessages();
      manager.disconnect();
      wsManagerRef.current = null;
    };
  }, []);

  return { isConnected, connectionState };
}

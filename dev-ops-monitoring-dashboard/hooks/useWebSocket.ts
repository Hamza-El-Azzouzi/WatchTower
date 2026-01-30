"use client";

import { useEffect, useRef, useState, useCallback } from "react";
import {
  WebSocketManager,
  createWebSocketManager,
  WsMessage,
  WsMetricMessage,
  WsLogMessage,
  WsAlertMessage,
} from "@/lib/websocket";

/**
 * Hook for real-time metrics updates - simplified, self-contained implementation
 */
export function useMetricsWebSocket(
  onMetric: (message: WsMetricMessage) => void
) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<string>("disconnected");
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  const onMetricRef = useRef(onMetric);
  
  // Keep callback ref updated
  useEffect(() => {
    onMetricRef.current = onMetric;
  }, [onMetric]);

  useEffect(() => {
    console.log("[useMetricsWebSocket] Initializing WebSocket connection...");
    
    // Create WebSocket manager
    const manager = createWebSocketManager("metrics");
    wsManagerRef.current = manager;

    // Subscribe to state changes
    const unsubscribeState = manager.onStateChange((state) => {
      console.log("[useMetricsWebSocket] Connection state:", state);
      setConnectionState(state);
      setIsConnected(state === "connected");
    });

    // Subscribe to messages
    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      if (typeof message !== "string" && "Metric" in message) {
        onMetricRef.current(message as WsMetricMessage);
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

  return { isConnected, connectionState };
}

/**
 * Hook for real-time log updates - simplified, self-contained implementation
 */
export function useLogsWebSocket(onLog: (message: WsLogMessage) => void) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<string>("disconnected");
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
      setConnectionState(state);
      setIsConnected(state === "connected");
    });

    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      if (typeof message !== "string" && "Log" in message) {
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
 * Hook for real-time alert updates - simplified, self-contained implementation
 */
export function useAlertsWebSocket(onAlert: (message: WsAlertMessage) => void) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<string>("disconnected");
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
      setConnectionState(state);
      setIsConnected(state === "connected");
    });

    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      if (typeof message !== "string" && "Alert" in message) {
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

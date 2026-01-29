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
 * Hook for managing WebSocket connection state
 */
export function useWebSocket(endpoint: "metrics" | "logs" | "alerts") {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<
    "connecting" | "connected" | "disconnected"
  >("disconnected");
  const wsManagerRef = useRef<WebSocketManager | null>(null);

  useEffect(() => {
    // Create WebSocket manager
    const manager = createWebSocketManager(endpoint);
    wsManagerRef.current = manager;

    // Subscribe to state changes
    const unsubscribeState = manager.onStateChange((state) => {
      setConnectionState(state as any);
      setIsConnected(state === "connected");
    });

    // Connect
    manager.connect();

    // Cleanup on unmount
    return () => {
      unsubscribeState();
      manager.disconnect();
      wsManagerRef.current = null;
    };
  }, [endpoint]);

  return {
    isConnected,
    connectionState,
    manager: wsManagerRef.current,
  };
}

/**
 * Hook for subscribing to WebSocket messages
 */
export function useWebSocketSubscription<T = WsMessage>(
  endpoint: "metrics" | "logs" | "alerts",
  callback: (message: T) => void,
  deps: React.DependencyList = []
) {
  const { isConnected, connectionState, manager } = useWebSocket(endpoint);

  useEffect(() => {
    if (!manager) return;

    const unsubscribe = manager.subscribe(callback as any);
    return unsubscribe;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [manager, ...deps]);

  return { isConnected, connectionState };
}

/**
 * Hook for real-time metrics updates
 */
export function useMetricsWebSocket(
  onMetric: (message: WsMetricMessage) => void
) {
  const [metrics, setMetrics] = useState<Map<string, WsMetricMessage>>(
    new Map()
  );

  const handleMessage = useCallback(
    (message: WsMessage) => {
      if ("Metric" in message) {
        const metricMessage = message as WsMetricMessage;
        const key = `${metricMessage.Metric.agent_id}:${metricMessage.Metric.metric_name}`;

        setMetrics((prev) => {
          const updated = new Map(prev);
          updated.set(key, metricMessage);
          return updated;
        });

        onMetric(metricMessage);
      }
    },
    [onMetric]
  );

  const { isConnected, connectionState } = useWebSocketSubscription(
    "metrics",
    handleMessage,
    [handleMessage]
  );

  return {
    metrics: Array.from(metrics.values()),
    isConnected,
    connectionState,
  };
}

/**
 * Hook for real-time log updates
 */
export function useLogsWebSocket(onLog: (message: WsLogMessage) => void) {
  const [logs, setLogs] = useState<WsLogMessage[]>([]);
  const maxLogs = 1000; // Keep last 1000 logs in memory

  const handleMessage = useCallback(
    (message: WsMessage) => {
      if ("Log" in message) {
        const logMessage = message as WsLogMessage;

        setLogs((prev) => {
          const updated = [logMessage, ...prev];
          return updated.slice(0, maxLogs);
        });

        onLog(logMessage);
      }
    },
    [onLog]
  );

  const { isConnected, connectionState } = useWebSocketSubscription(
    "logs",
    handleMessage,
    [handleMessage]
  );

  const clearLogs = useCallback(() => {
    setLogs([]);
  }, []);

  return {
    logs,
    clearLogs,
    isConnected,
    connectionState,
  };
}

/**
 * Hook for real-time alert updates
 */
export function useAlertsWebSocket(onAlert: (message: WsAlertMessage) => void) {
  const [alerts, setAlerts] = useState<Map<number, WsAlertMessage>>(new Map());

  const handleMessage = useCallback(
    (message: WsMessage) => {
      if ("Alert" in message) {
        const alertMessage = message as WsAlertMessage;

        setAlerts((prev) => {
          const updated = new Map(prev);
          updated.set(alertMessage.Alert.alert_id, alertMessage);
          return updated;
        });

        onAlert(alertMessage);
      }
    },
    [onAlert]
  );

  const { isConnected, connectionState } = useWebSocketSubscription(
    "alerts",
    handleMessage,
    [handleMessage]
  );

  return {
    alerts: Array.from(alerts.values()),
    isConnected,
    connectionState,
  };
}

/**
 * Hook for displaying connection status
 */
export function useConnectionStatus(endpoint: "metrics" | "logs" | "alerts") {
  const { isConnected, connectionState } = useWebSocket(endpoint);

  const statusText = {
    connected: "Connected",
    connecting: "Connecting...",
    disconnected: "Disconnected",
  }[connectionState];

  const statusColor = {
    connected: "green",
    connecting: "yellow",
    disconnected: "red",
  }[connectionState];

  return {
    isConnected,
    connectionState,
    statusText,
    statusColor,
  };
}

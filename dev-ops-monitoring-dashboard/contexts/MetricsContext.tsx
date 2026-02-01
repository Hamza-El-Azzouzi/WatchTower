'use client';

import React, { createContext, useContext, useState, useEffect, useRef, useMemo, useCallback } from 'react';
import { 
  WebSocketManager, 
  createWebSocketManager, 
  WsMessage,
  WsMetricMessage, 
  WsAgentSnapshot, 
  WsMetricSnapshot, 
  WsInitialStateMessage
} from '@/lib/websocket';
import { Agent, LatestMetrics, Metric } from '@/types';

// Callback types for raw metric events (for charts)
type MetricCallback = (message: WsMetricMessage) => void;
type InitialStateCallback = (agents: WsAgentSnapshot[], metrics: WsMetricSnapshot[]) => void;

interface MetricsContextValue {
  // Connection state
  isConnected: boolean;
  connectionState: 'connecting' | 'connected' | 'disconnected';
  initialStateReceived: boolean;
  
  // Data from WebSocket
  agents: Agent[];
  agentMetrics: Record<string, LatestMetrics>;
  
  // Last update timestamp
  lastUpdated: Date;
  
  // Subscription methods for charts (raw metric events)
  subscribeToMetrics: (callback: MetricCallback) => () => void;
  subscribeToInitialState: (callback: InitialStateCallback) => () => void;
}

const MetricsContext = createContext<MetricsContextValue | null>(null);

export function useMetricsContext() {
  const context = useContext(MetricsContext);
  if (!context) {
    throw new Error('useMetricsContext must be used within a MetricsProvider');
  }
  return context;
}

interface MetricsProviderProps {
  children: React.ReactNode;
}

export function MetricsProvider({ children }: MetricsProviderProps) {
  const [isConnected, setIsConnected] = useState(false);
  const [connectionState, setConnectionState] = useState<'connecting' | 'connected' | 'disconnected'>('disconnected');
  const [initialStateReceived, setInitialStateReceived] = useState(false);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [agentMetrics, setAgentMetrics] = useState<Record<string, LatestMetrics>>({});
  const [lastUpdated, setLastUpdated] = useState(new Date());
  
  const wsManagerRef = useRef<WebSocketManager | null>(null);
  
  // Subscription refs for charts
  const metricSubscribersRef = useRef<Set<MetricCallback>>(new Set());
  const initialStateSubscribersRef = useRef<Set<InitialStateCallback>>(new Set());
  
  // Cached initial state for late subscribers
  const cachedInitialStateRef = useRef<{ agents: WsAgentSnapshot[], metrics: WsMetricSnapshot[] } | null>(null);
  
  // Subscribe to raw metric events (for charts)
  const subscribeToMetrics = useCallback((callback: MetricCallback) => {
    metricSubscribersRef.current.add(callback);
    return () => {
      metricSubscribersRef.current.delete(callback);
    };
  }, []);
  
  // Subscribe to initial state (for charts) - also delivers cached state if available
  const subscribeToInitialState = useCallback((callback: InitialStateCallback) => {
    initialStateSubscribersRef.current.add(callback);
    
    // If we already have initial state, deliver it immediately
    if (cachedInitialStateRef.current) {
      callback(cachedInitialStateRef.current.agents, cachedInitialStateRef.current.metrics);
    }
    
    return () => {
      initialStateSubscribersRef.current.delete(callback);
    };
  }, []);
  
  useEffect(() => {
    console.log('[MetricsProvider] Initializing shared WebSocket connection...');
    
    const manager = createWebSocketManager('metrics');
    wsManagerRef.current = manager;
    
    // Subscribe to state changes
    const unsubscribeState = manager.onStateChange((state) => {
      console.log('[MetricsProvider] Connection state:', state);
      setConnectionState(state as 'connecting' | 'connected' | 'disconnected');
      setIsConnected(state === 'connected');
      
      if (state === 'disconnected') {
        setInitialStateReceived(false);
        cachedInitialStateRef.current = null;
      }
    });
    
    // Subscribe to messages
    const unsubscribeMessages = manager.subscribe((message: WsMessage) => {
      // Handle initial state
      if (message.type === 'initial_state') {
        const initMsg = message as WsInitialStateMessage;
        console.log('[MetricsProvider] Received initial state:', initMsg.agents.length, 'agents,', initMsg.metrics.length, 'metrics');
        
        // Cache initial state for late subscribers
        cachedInitialStateRef.current = { agents: initMsg.agents, metrics: initMsg.metrics };
        
        // Notify initial state subscribers (charts)
        initialStateSubscribersRef.current.forEach(cb => cb(initMsg.agents, initMsg.metrics));
        
        // Map agents
        const mappedAgents: Agent[] = initMsg.agents.map(a => ({
          id: a.id,
          name: a.name,
          status: a.status as 'Healthy' | 'Degraded' | 'Unreachable',
          last_seen: a.last_seen,
        }));
        setAgents(mappedAgents);
        console.log('[MetricsProvider] Mapped agents:', mappedAgents);
        
        // Group metrics by agent
        const metricsMap: Record<string, LatestMetrics> = {};
        initMsg.metrics.forEach(m => {
          if (!metricsMap[m.agent_id]) {
            metricsMap[m.agent_id] = {
              agent_id: m.agent_id,
              metrics: [],
            };
          }
          metricsMap[m.agent_id].metrics.push({
            name: m.metric_name,
            value: m.latest_value,
            timestamp: m.timestamp,
          });
        });
        console.log('[MetricsProvider] Metrics map keys:', Object.keys(metricsMap));
        console.log('[MetricsProvider] Sample metrics for first agent:', Object.values(metricsMap)[0]?.metrics.slice(0, 5));
        setAgentMetrics(metricsMap);
        
        setInitialStateReceived(true);
        setLastUpdated(new Date());
        return;
      }
      
      // Handle real-time metric updates
      if (message.type === 'metric') {
        const metricMsg = message as WsMetricMessage;
        
        // Notify metric subscribers (charts)
        metricSubscribersRef.current.forEach(cb => cb(metricMsg));
        
        // Update agent last_seen
        setAgents(prev => prev.map(agent => 
          agent.id === metricMsg.agent_id
            ? { ...agent, last_seen: metricMsg.timestamp }
            : agent
        ));
        
        // Update metrics
        setAgentMetrics(prev => {
          const agentId = metricMsg.agent_id;
          const existing = prev[agentId];
          
          if (!existing) {
            return {
              ...prev,
              [agentId]: {
                agent_id: agentId,
                metrics: [{
                  name: metricMsg.metric_name,
                  value: metricMsg.value,
                  timestamp: metricMsg.timestamp,
                }],
              },
            };
          }
          
          const existingIdx = existing.metrics.findIndex(m => m.name === metricMsg.metric_name);
          const updatedMetrics = [...existing.metrics];
          
          if (existingIdx >= 0) {
            updatedMetrics[existingIdx] = {
              name: metricMsg.metric_name,
              value: metricMsg.value,
              timestamp: metricMsg.timestamp,
            };
          } else {
            updatedMetrics.push({
              name: metricMsg.metric_name,
              value: metricMsg.value,
              timestamp: metricMsg.timestamp,
            });
          }
          
          return {
            ...prev,
            [agentId]: { ...existing, metrics: updatedMetrics },
          };
        });
        
        setLastUpdated(new Date());
      }
    });
    
    // Connect
    manager.connect();
    
    // Cleanup
    return () => {
      console.log('[MetricsProvider] Cleaning up shared WebSocket...');
      unsubscribeState();
      unsubscribeMessages();
      manager.disconnect();
    };
  }, []);
  
  // Memoize the context value to prevent unnecessary rerenders
  const value = useMemo<MetricsContextValue>(() => ({
    isConnected,
    connectionState,
    initialStateReceived,
    agents,
    agentMetrics,
    lastUpdated,
    subscribeToMetrics,
    subscribeToInitialState,
  }), [isConnected, connectionState, initialStateReceived, agents, agentMetrics, lastUpdated, subscribeToMetrics, subscribeToInitialState]);
  
  return (
    <MetricsContext.Provider value={value}>
      {children}
    </MetricsContext.Provider>
  );
}

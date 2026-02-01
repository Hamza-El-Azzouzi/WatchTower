'use client';

import { useState, useEffect, useRef } from 'react';

export default function WebSocketTestPage() {
  const [messages, setMessages] = useState<string[]>([]);
  const [status, setStatus] = useState<string>('disconnected');
  const [reconnectCount, setReconnectCount] = useState(0);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    let reconnectTimeout: NodeJS.Timeout | null = null;
    let shouldReconnect = true;
    
    const connect = () => {
      const url = 'ws://localhost:8080/api/v1/ws/metrics';
      console.log('[WS-Test] Connecting to:', url, 'attempt:', reconnectCount + 1);
      setStatus('connecting');
      
      const ws = new WebSocket(url);
      wsRef.current = ws;
      
      ws.onopen = () => {
        console.log('[WS-Test] Connected!');
        setStatus('connected');
        setMessages(prev => [...prev.slice(-49), `[${new Date().toLocaleTimeString()}] ✅ CONNECTED`]);
      };
      
      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          console.log('[WS-Test] Received:', data.type);
          
          let summary: string;
          if (data.type === 'initial_state') {
            summary = `initial_state: ${data.agents?.length || 0} agents, ${data.metrics?.length || 0} metrics`;
          } else if (data.type === 'metric') {
            summary = `metric: ${data.agent_id} - ${data.metric_name} = ${data.value?.toFixed(2)}`;
          } else if (data.type === 'heartbeat') {
            summary = '💓 heartbeat';
          } else {
            summary = `${data.type}: ${JSON.stringify(data).slice(0, 80)}...`;
          }
          
          setMessages(prev => {
            const newMessages = [...prev, `[${new Date().toLocaleTimeString()}] ${summary}`];
            if (newMessages.length > 100) {
              return newMessages.slice(-100);
            }
            return newMessages;
          });
        } catch (e) {
          console.error('[WS-Test] Parse error:', e);
          setMessages(prev => [...prev.slice(-49), `[${new Date().toLocaleTimeString()}] ❌ Parse error`]);
        }
      };
      
      ws.onerror = (event) => {
        console.error('[WS-Test] Error:', event);
        setMessages(prev => [...prev.slice(-49), `[${new Date().toLocaleTimeString()}] ❌ WebSocket error`]);
      };
      
      ws.onclose = (event) => {
        console.log('[WS-Test] Closed:', event.code, event.reason, 'wasClean:', event.wasClean);
        setStatus('disconnected');
        setMessages(prev => [...prev.slice(-49), `[${new Date().toLocaleTimeString()}] 🔌 CLOSED: code=${event.code}, wasClean=${event.wasClean}, reason=${event.reason || 'none'}`]);
        
        if (shouldReconnect) {
          console.log('[WS-Test] Will reconnect in 2s...');
          reconnectTimeout = setTimeout(() => {
            setReconnectCount(c => c + 1);
            connect();
          }, 2000);
        }
      };
    };
    
    connect();
    
    return () => {
      console.log('[WS-Test] Cleanup - closing connection');
      shouldReconnect = false;
      if (reconnectTimeout) {
        clearTimeout(reconnectTimeout);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, []);

  return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
      <h1 className="text-2xl font-bold mb-4">Raw WebSocket Test</h1>
      
      <div className="mb-4 flex gap-4 items-center">
        <span className="font-semibold">Status: </span>
        <span className={`px-3 py-1 rounded ${status === 'connected' ? 'bg-green-600' : status === 'connecting' ? 'bg-yellow-600' : 'bg-red-600'}`}>
          {status}
        </span>
        <span className="text-gray-400">Reconnects: {reconnectCount}</span>
      </div>
      
      <div className="mb-4 text-sm text-gray-400">
        URL: ws://localhost:8080/api/v1/ws/metrics
      </div>
      
      <div className="bg-gray-800 p-4 rounded-lg max-h-[600px] overflow-y-auto font-mono text-xs">
        {messages.length === 0 ? (
          <p className="text-gray-400">Waiting for messages...</p>
        ) : (
          messages.map((msg, i) => (
            <div key={i} className="text-gray-200 py-0.5 border-b border-gray-700/50 break-all">
              {msg}
            </div>
          ))
        )}
      </div>
      
      <div className="mt-4 text-sm text-gray-500">
        <p>Open browser console (F12) to see detailed logs.</p>
        <p>This uses a raw WebSocket, not the MetricsContext.</p>
      </div>
    </div>
  );
}

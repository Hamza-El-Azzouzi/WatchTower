'use client'

import { useEffect, useState, useCallback } from 'react'
import { useMetricsWebSocket } from '@/hooks/useWebSocket'
import { WsMetricMessage } from '@/lib/websocket'

export default function WebSocketTestPage() {
  const [messages, setMessages] = useState<string[]>([])
  
  const handleMetric = useCallback((msg: WsMetricMessage) => {
    const logEntry = `[${new Date().toISOString()}] ${msg.Metric.agent_id}: ${msg.Metric.metric_name} = ${msg.Metric.value}`
    console.log('[WS Test]', logEntry)
    setMessages(prev => [logEntry, ...prev].slice(0, 50))
  }, [])
  
  const { isConnected, connectionState } = useMetricsWebSocket(handleMetric)
  
  useEffect(() => {
    console.log('[WS Test Page] Mounted')
    return () => console.log('[WS Test Page] Unmounted')
  }, [])

  return (
    <div className="min-h-screen bg-gray-900 text-white p-8">
      <h1 className="text-2xl font-bold mb-4">WebSocket Test Page</h1>
      
      <div className="mb-4">
        <span className="font-semibold">Connection Status: </span>
        <span className={isConnected ? 'text-green-400' : 'text-red-400'}>
          {connectionState} {isConnected ? '✓' : '✗'}
        </span>
      </div>
      
      <div className="mb-4">
        <h2 className="text-lg font-semibold mb-2">Recent Messages ({messages.length})</h2>
        <div className="bg-gray-800 p-4 rounded h-96 overflow-auto font-mono text-sm">
          {messages.length === 0 ? (
            <div className="text-gray-500">Waiting for WebSocket messages...</div>
          ) : (
            messages.map((msg, i) => (
              <div key={i} className="py-1 border-b border-gray-700">{msg}</div>
            ))
          )}
        </div>
      </div>
      
      <div className="text-sm text-gray-500">
        <p>Open browser console (F12) to see detailed WebSocket logs.</p>
        <p>Expected URL: ws://localhost:8080/api/v1/ws/metrics</p>
      </div>
    </div>
  )
}

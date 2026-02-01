'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  TooltipProps,
} from 'recharts';
import { formatChartTime } from '@/lib/metrics-utils';
import { useMetricsContext } from '@/contexts/MetricsContext';
import { WsMetricMessage, WsMetricSnapshot, WsAgentSnapshot } from '@/lib/websocket';

interface ChartsSectionProps {
  agentId: string;
}

interface ChartData {
  timestamp: string;
  rawTimestamp?: string;
  [key: string]: string | number | undefined;
}

// Custom tooltip component for displaying exact values
const CustomTooltip = ({ active, payload, label }: TooltipProps<number, string>) => {
  if (active && payload && payload.length) {
    return (
      <div style={{
        backgroundColor: 'rgba(26,26,26,0.95)',
        border: '1px solid rgba(99,102,241,0.3)',
        borderRadius: '8px',
        padding: '12px',
        color: '#f5f5f5'
      }}>
        <p style={{ marginBottom: '8px', fontWeight: 'bold' }}>{label}</p>
        {payload.map((entry, index) => (
          <p key={index} style={{ color: entry.color, margin: '4px 0' }}>
            {entry.name}: {typeof entry.value === 'number' ? entry.value.toFixed(2) : entry.value}
            {entry.dataKey && String(entry.dataKey).includes('core') ? '%' : ''}
            {entry.dataKey === 'cpu' ? '%' : ''}
            {entry.dataKey === 'memory' || entry.dataKey === 'swap' ? '%' : ''}
            {entry.dataKey === 'usage' ? '%' : ''}
            {entry.dataKey === 'rx' || entry.dataKey === 'tx' ? ' MB' : ''}
          </p>
        ))}
      </div>
    );
  }
  return null;
};

export default function ChartsSection({ agentId }: ChartsSectionProps) {
  const [cpuData, setCpuData] = useState<ChartData[]>([]);
  const [cpuCoreData, setCpuCoreData] = useState<ChartData[]>([]);
  const [coreCount, setCoreCount] = useState<number>(0);
  const [memoryData, setMemoryData] = useState<ChartData[]>([]);
  const [diskData, setDiskData] = useState<ChartData[]>([]);
  const [networkData, setNetworkData] = useState<ChartData[]>([]);
  const [isLive, setIsLive] = useState(true);
  
  // Track last update time to throttle updates
  const lastUpdateRef = useRef<number>(0);
  const maxDataPoints = 100; // Maximum points to show

  // Handle initial state from WebSocket
  const handleInitialState = useCallback((agents: WsAgentSnapshot[], metrics: WsMetricSnapshot[]) => {
    console.log('[ChartsSection] Received initial state for', agentId);
    
    // Filter metrics for this agent
    const agentMetrics = metrics.filter(m => m.agent_id === agentId);
    
    // Detect CPU cores from initial state
    const coreMetrics = agentMetrics.filter(m => m.metric_name.startsWith('cpu_core_'));
    if (coreMetrics.length > 0) {
      setCoreCount(coreMetrics.length);
    }
    
    // Initialize charts with latest values from initial state
    const now = new Date().toISOString();
    const timestamp = formatChartTime(now);
    
    // CPU
    const cpuMetric = agentMetrics.find(m => m.metric_name === 'cpu_usage');
    if (cpuMetric) {
      setCpuData([{ timestamp, rawTimestamp: now, cpu: cpuMetric.latest_value }]);
    }
    
    // Memory
    const memMetric = agentMetrics.find(m => m.metric_name === 'memory_usage');
    const swapMetric = agentMetrics.find(m => m.metric_name === 'swap_usage');
    if (memMetric) {
      setMemoryData([{ 
        timestamp, 
        rawTimestamp: now, 
        memory: memMetric.latest_value,
        swap: swapMetric?.latest_value ?? 0
      }]);
    }
    
    // Disk
    const diskMetric = agentMetrics.find(m => m.metric_name === 'disk_usage');
    if (diskMetric) {
      setDiskData([{ timestamp, rawTimestamp: now, usage: diskMetric.latest_value }]);
    }
    
    // Network
    const rxMetric = agentMetrics.find(m => m.metric_name === 'network_rx_bytes');
    const txMetric = agentMetrics.find(m => m.metric_name === 'network_tx_bytes');
    if (rxMetric || txMetric) {
      setNetworkData([{ 
        timestamp, 
        rawTimestamp: now, 
        rx: (rxMetric?.latest_value ?? 0) / (1024 * 1024),
        tx: (txMetric?.latest_value ?? 0) / (1024 * 1024)
      }]);
    }
    
    // Per-core CPU
    if (coreMetrics.length > 0) {
      const coreData: ChartData = { timestamp, rawTimestamp: now };
      coreMetrics.forEach(m => {
        const coreIdx = parseInt(m.metric_name.split('_')[2], 10);
        coreData[`core${coreIdx}`] = m.latest_value;
      });
      setCpuCoreData([coreData]);
    }
  }, [agentId]);

  // Handle real-time metric updates from WebSocket
  const handleMetricUpdate = useCallback((metricMessage: WsMetricMessage) => {
    if (metricMessage.agent_id !== agentId) return;
    if (!isLive) return;

    const rawTimestamp = metricMessage.timestamp;
    const timestamp = formatChartTime(rawTimestamp);
    const metricName = metricMessage.metric_name;
    const value = metricMessage.value;
    
    if (!metricName) return;

    // Per-core CPU updates - no throttling, group by time window
    if (metricName.startsWith('cpu_core_')) {
      const coreIdx = parseInt(metricName.split('_')[2], 10);
      setCoreCount(prev => Math.max(prev, coreIdx + 1));
      
      setCpuCoreData(prev => {
        // Use the last data point if within 2 seconds, otherwise create new
        const now = Date.now();
        const lastPoint = prev[prev.length - 1];
        const lastPointTime = lastPoint?.rawTimestamp ? new Date(lastPoint.rawTimestamp).getTime() : 0;
        const withinWindow = (now - lastPointTime) < 2000;
        
        if (withinWindow && lastPoint) {
          // Update the last point with this core's value
          const updated = [...prev];
          updated[updated.length - 1] = { 
            ...lastPoint, 
            [`core${coreIdx}`]: value,
            timestamp, // Update timestamp to latest
            rawTimestamp 
          };
          return updated;
        } else {
          // Create new point with this core
          const newPoint: ChartData = { timestamp, rawTimestamp, [`core${coreIdx}`]: value };
          // Copy previous core values to maintain continuity
          if (lastPoint) {
            for (let i = 0; i < 16; i++) {
              if (lastPoint[`core${i}`] !== undefined && i !== coreIdx) {
                newPoint[`core${i}`] = lastPoint[`core${i}`];
              }
            }
          }
          const updated = [...prev, newPoint];
          return updated.slice(-maxDataPoints);
        }
      });
      return; // Don't apply global throttle for core updates
    }
    
    // Update CPU data - no throttling for main CPU
    if (metricName === 'cpu_usage') {
      setCpuData(prev => {
        const newPoint: ChartData = { timestamp, rawTimestamp, cpu: value };
        const updated = [...prev, newPoint];
        return updated.slice(-maxDataPoints);
      });
      return;
    }
    
    // Update Memory data - no throttling
    if (metricName === 'memory_usage') {
      setMemoryData(prev => {
        const last = prev[prev.length - 1];
        const newPoint: ChartData = { 
          timestamp, 
          rawTimestamp, 
          memory: value, 
          swap: last?.swap ?? 0 
        };
        const updated = [...prev, newPoint];
        return updated.slice(-maxDataPoints);
      });
      return;
    }
    
    if (metricName === 'swap_usage') {
      setMemoryData(prev => {
        if (prev.length === 0) return [{ timestamp, rawTimestamp, memory: 0, swap: value }];
        const last = { ...prev[prev.length - 1] };
        last.swap = value;
        last.timestamp = timestamp;
        last.rawTimestamp = rawTimestamp;
        return [...prev.slice(0, -1), last];
      });
      return;
    }

    // Update Network data - no throttling
    if (metricName === 'network_rx_bytes') {
      setNetworkData(prev => {
        const last = prev[prev.length - 1];
        const newPoint: ChartData = { 
          timestamp, 
          rawTimestamp, 
          rx: value / (1024 * 1024),
          tx: last?.tx ?? 0 
        };
        const updated = [...prev, newPoint];
        return updated.slice(-maxDataPoints);
      });
      return;
    }
    
    if (metricName === 'network_tx_bytes') {
      setNetworkData(prev => {
        if (prev.length === 0) return [{ timestamp, rawTimestamp, rx: 0, tx: value / (1024 * 1024) }];
        const last = { ...prev[prev.length - 1] };
        last.tx = value / (1024 * 1024);
        last.timestamp = timestamp;
        last.rawTimestamp = rawTimestamp;
        return [...prev.slice(0, -1), last];
      });
      return;
    }

    // Update Disk data - no throttling
    if (metricName === 'disk_usage') {
      setDiskData(prev => {
        const newPoint: ChartData = { timestamp, rawTimestamp, usage: value };
        const updated = [...prev, newPoint];
        return updated.slice(-maxDataPoints);
      });
      return;
    }
  }, [agentId, isLive]);

  // Use the shared WebSocket context - single connection for all components
  const { 
    isConnected, 
    connectionState, 
    initialStateReceived,
    subscribeToMetrics,
    subscribeToInitialState 
  } = useMetricsContext();
  
  // Subscribe to metric updates from shared context
  useEffect(() => {
    const unsubscribeMetrics = subscribeToMetrics(handleMetricUpdate);
    const unsubscribeInitial = subscribeToInitialState(handleInitialState);
    
    return () => {
      unsubscribeMetrics();
      unsubscribeInitial();
    };
  }, [subscribeToMetrics, subscribeToInitialState, handleMetricUpdate, handleInitialState]);

  const skeletonLoader = (
    <div className="w-full h-80 bg-background/30 rounded-lg animate-pulse flex items-center justify-center">
      <span className="text-muted-foreground">Waiting for data...</span>
    </div>
  );

  const hasData = cpuData.length > 0 || memoryData.length > 0 || diskData.length > 0;

  return (
    <div>
      {/* Connection Status & Live Controls */}
      <div className="flex items-center gap-4 mb-6 flex-wrap">
        {/* Connection Status */}
        <div className={`flex items-center gap-2 px-4 py-2 rounded-lg ${
          connectionState === 'connected' 
            ? 'bg-green-500/20 border border-green-500/30' 
            : connectionState === 'connecting'
            ? 'bg-yellow-500/20 border border-yellow-500/30'
            : 'bg-red-500/20 border border-red-500/30'
        }`}>
          <span className={`w-2 h-2 rounded-full ${
            connectionState === 'connected' 
              ? 'bg-green-400 animate-pulse' 
              : connectionState === 'connecting'
              ? 'bg-yellow-400 animate-pulse'
              : 'bg-red-400'
          }`} />
          <span className={`text-sm font-medium ${
            connectionState === 'connected' 
              ? 'text-green-400' 
              : connectionState === 'connecting'
              ? 'text-yellow-400'
              : 'text-red-400'
          }`}>
            {connectionState === 'connected' ? 'WebSocket Connected' : 
             connectionState === 'connecting' ? 'Connecting...' : 'Disconnected'}
          </span>
        </div>
        
        {/* Live/Pause Toggle */}
        <div className="flex items-center gap-2 ml-auto">
          <button
            onClick={() => setIsLive(!isLive)}
            disabled={!isConnected}
            className={`px-4 py-2 rounded-lg font-medium transition-smooth flex items-center gap-2 ${
              isLive && isConnected
                ? 'bg-green-600 text-white shadow-lg shadow-green-600/30'
                : 'glass-morphism text-muted-foreground hover:bg-card border-border'
            } ${!isConnected ? 'opacity-50 cursor-not-allowed' : ''}`}
          >
            <span className={`w-2 h-2 rounded-full ${isLive && isConnected ? 'bg-green-300 animate-pulse' : 'bg-gray-400'}`} />
            {isLive ? 'LIVE' : 'Paused'}
          </button>
          
          {initialStateReceived && (
            <span className="text-xs text-muted-foreground">
              {cpuData.length} data points
            </span>
          )}
        </div>
      </div>

      {/* No Connection Warning */}
      {connectionState === 'disconnected' && (
        <div className="text-yellow-400 text-sm mb-6 glass-morphism border border-yellow-500/30 bg-yellow-500/10 rounded-xl p-4 animate-slide-up">
          WebSocket disconnected. Attempting to reconnect...
        </div>
      )}

      {/* Waiting for Initial State */}
      {isConnected && !initialStateReceived && (
        <div className="text-blue-400 text-sm mb-6 glass-morphism border border-blue-500/30 bg-blue-500/10 rounded-xl p-4 animate-slide-up">
          Waiting for initial data from server...
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Per-Core CPU Chart - Full Width */}
        {cpuCoreData.length > 0 && coreCount > 0 && (
          <div className="lg:col-span-2 glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up">
            <h3 className="text-lg font-semibold text-foreground mb-4">
              Per-Core CPU Usage ({coreCount} cores) - Real-time
            </h3>
            <ResponsiveContainer width="100%" height={400}>
              <LineChart data={cpuCoreData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip content={<CustomTooltip />} />
                <Legend wrapperStyle={{ color: '#9ca3af' }} />
                {Array.from({ length: coreCount }, (_, i) => {
                  const colors = [
                    '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899',
                    '#14b8a6', '#f97316', '#06b6d4', '#84cc16', '#a855f7', '#f43f5e'
                  ];
                  return (
                    <Line
                      key={i}
                      type="monotone"
                      dataKey={`core${i}`}
                      name={`CPU${i+1}`}
                      stroke={colors[i % colors.length]}
                      dot={false}
                      strokeWidth={1.5}
                      isAnimationActive={true}
                      animationDuration={300}
                      animationEasing="ease-in-out"
                      connectNulls={true}
                    />
                  );
                })}
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}

        {/* CPU Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up">
          <h3 className="text-lg font-semibold text-foreground mb-4">CPU Usage - Real-time</h3>
          {cpuData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={cpuData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip content={<CustomTooltip />} />
                <Line
                  type="monotone"
                  dataKey="cpu"
                  name="CPU %"
                  stroke="#3b82f6"
                  fill="url(#cpuGradient)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            skeletonLoader
          )}
        </div>

        {/* Memory Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '50ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Memory & Swap - Real-time</h3>
          {memoryData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={memoryData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip content={<CustomTooltip />} />
                <Legend wrapperStyle={{ color: '#9ca3af' }} />
                <Area
                  type="monotone"
                  dataKey="memory"
                  name="Memory"
                  stroke="#10b981"
                  fill="#10b981"
                  fillOpacity={0.3}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
                <Area
                  type="monotone"
                  dataKey="swap"
                  name="Swap"
                  stroke="#8b5cf6"
                  fill="#8b5cf6"
                  fillOpacity={0.3}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            skeletonLoader
          )}
        </div>

        {/* Disk Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '100ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Disk Usage - Real-time</h3>
          {diskData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={diskData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip content={<CustomTooltip />} />
                <Area
                  type="monotone"
                  dataKey="usage"
                  name="Disk Usage"
                  stroke="#f59e0b"
                  fill="#f59e0b"
                  fillOpacity={0.3}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            skeletonLoader
          )}
        </div>

        {/* Network Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '150ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Network Traffic - Real-time</h3>
          {networkData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={networkData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} label={{ value: 'MB', angle: -90, position: 'insideLeft' }} />
                <Tooltip content={<CustomTooltip />} />
                <Legend wrapperStyle={{ color: '#9ca3af' }} />
                <Line
                  type="monotone"
                  dataKey="rx"
                  stroke="#3b82f6"
                  name="RX (Download)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
                <Line
                  type="monotone"
                  dataKey="tx"
                  stroke="#10b981"
                  name="TX (Upload)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={true}
                  animationDuration={300}
                  animationEasing="ease-in-out"
                />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            skeletonLoader
          )}
        </div>
      </div>
    </div>
  );
}

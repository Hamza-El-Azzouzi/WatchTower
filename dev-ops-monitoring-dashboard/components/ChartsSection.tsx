'use client';

import { useEffect, useState } from 'react';
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
} from 'recharts';
import { getHistoricalMetrics } from '@/lib/api';
import { formatChartTime, formatBytes } from '@/lib/metrics-utils';
import { HistoricalMetrics, DataPoint } from '@/types';

interface ChartsSectionProps {
  agentId: string;
}

interface ChartData {
  timestamp: string;
  [key: string]: string | number;
}

export default function ChartsSection({ agentId }: ChartsSectionProps) {
  const [cpuData, setCpuData] = useState<ChartData[]>([]);
  const [memoryData, setMemoryData] = useState<ChartData[]>([]);
  const [diskData, setDiskData] = useState<ChartData[]>([]);
  const [networkData, setNetworkData] = useState<ChartData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | '7d'>('1h');

  const getLimitForTimeRange = (range: typeof timeRange): number => {
    switch (range) {
      case '1h':
        return 360; // 1 data point per 10 seconds
      case '6h':
        return 2160; // 1 data point per 10 seconds
      case '24h':
        return 8640;
      case '7d':
        return 60480;
      default:
        return 360;
    }
  };

  useEffect(() => {
    let isInitialLoad = true;

    const fetchChartData = async () => {
      try {
        // Only show loading on initial mount, not refreshes
        if (isInitialLoad) {
          setLoading(true);
        }
        
        const limit = getLimitForTimeRange(timeRange);

        // Fetch all metric data in parallel
        const [cpuRes, memoryRes, diskRes, networkRxRes, networkTxRes] = await Promise.all([
          getHistoricalMetrics(agentId, 'cpu_usage', limit),
          getHistoricalMetrics(agentId, 'memory_usage', limit),
          getHistoricalMetrics(agentId, 'disk_usage', limit),
          getHistoricalMetrics(agentId, 'network_rx_bytes', limit),
          getHistoricalMetrics(agentId, 'network_tx_bytes', limit),
        ]);

        // Process CPU data
        const cpuChartData = cpuRes.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          cpu: Number(point.value.toFixed(1)),
        }));
        setCpuData(cpuChartData);

        // Process Memory data (percentage values)
        const memoryChartData = memoryRes.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          usage: Number(point.value.toFixed(2)),
        }));
        setMemoryData(memoryChartData);

        // Process Disk data (percentage values)
        const diskChartData = diskRes.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          usage: Number(point.value.toFixed(2)),
        }));
        setDiskData(diskChartData);

        // Process Network data
        const networkChartData = networkRxRes.data_points.map((point: DataPoint, idx: number) => ({
          timestamp: formatChartTime(point.timestamp),
          rx: Number((point.value / (1024 * 1024)).toFixed(2)),
          tx: Number(
            (networkTxRes.data_points[idx]?.value / (1024 * 1024) || 0).toFixed(2)
          ),
        }));
        setNetworkData(networkChartData);

        setError(null);
      } catch {
        setError('Failed to load chart data');
      } finally {
        if (isInitialLoad) {
          setLoading(false);
          isInitialLoad = false;
        }
      }
    };

    fetchChartData();
    
    // Auto-refresh every 10 seconds
    const interval = setInterval(fetchChartData, 10000);
    return () => clearInterval(interval);
  }, [agentId, timeRange]);

  const timeRangeButtons = ['1h', '6h', '24h', '7d'] as const;

  const skeletonLoader = (
    <div className="w-full h-80 bg-background/30 rounded-lg animate-pulse" />
  );

  return (
    <div>
      {/* Time Range Selector */}
      <div className="flex gap-2 mb-6 flex-wrap">
        {timeRangeButtons.map(range => (
          <button
            key={range}
            onClick={() => setTimeRange(range)}
            className={`px-4 py-2 rounded-lg font-medium transition-smooth ${
              timeRange === range
                ? 'bg-primary text-primary-foreground shadow-lg'
                : 'glass-morphism text-muted-foreground hover:bg-card border-border'
            }`}
          >
            {range === '1h' ? '1 Hour' : range === '6h' ? '6 Hours' : range === '24h' ? '24 Hours' : '7 Days'}
          </button>
        ))}
      </div>

      {error && (
        <div className="text-red-400 text-sm mb-6 glass-morphism border border-red-500/30 bg-red-500/10 rounded-xl p-4 animate-slide-up">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* CPU Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up">
          <h3 className="text-lg font-semibold text-foreground mb-4">CPU Usage Over Time</h3>
          {loading ? (
            skeletonLoader
          ) : cpuData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={cpuData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip contentStyle={{ backgroundColor: 'rgba(26,26,26,0.95)', border: '1px solid rgba(99,102,241,0.3)', borderRadius: '8px', color: '#f5f5f5' }} />
                <Line
                  type="monotone"
                  dataKey="cpu"
                  stroke="#3b82f6"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={true}
                  animationDuration={800}
                  animationEasing="ease-in-out"
                />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <p className="text-muted-foreground">No data available</p>
          )}
        </div>

        {/* Memory Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '50ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Memory Usage Over Time</h3>
          {loading ? (
            skeletonLoader
          ) : memoryData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={memoryData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip contentStyle={{ backgroundColor: 'rgba(26,26,26,0.95)', border: '1px solid rgba(99,102,241,0.3)', borderRadius: '8px', color: '#f5f5f5' }} />
                <Area
                  type="monotone"
                  dataKey="usage"
                  stroke="#10b981"
                  fill="#10b981"
                  fillOpacity={0.3}
                  isAnimationActive={true}
                  animationDuration={800}
                  animationEasing="ease-in-out"
                />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <p className="text-muted-foreground">No data available</p>
          )}
        </div>

        {/* Disk Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '100ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Disk Usage Over Time</h3>
          {loading ? (
            skeletonLoader
          ) : diskData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <AreaChart data={diskData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} label={{ value: '%', angle: -90, position: 'insideLeft' }} />
                <Tooltip contentStyle={{ backgroundColor: 'rgba(26,26,26,0.95)', border: '1px solid rgba(99,102,241,0.3)', borderRadius: '8px', color: '#f5f5f5' }} />
                <Area
                  type="monotone"
                  dataKey="usage"
                  stroke="#f59e0b"
                  fill="#f59e0b"
                  fillOpacity={0.3}
                  isAnimationActive={true}
                  animationDuration={800}
                  animationEasing="ease-in-out"
                />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <p className="text-muted-foreground">No data available</p>
          )}
        </div>

        {/* Network Chart */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up" style={{ animationDelay: '150ms' }}>
          <h3 className="text-lg font-semibold text-foreground mb-4">Network Traffic Over Time</h3>
          {loading ? (
            skeletonLoader
          ) : networkData.length > 0 ? (
            <ResponsiveContainer width="100%" height={300}>
              <LineChart data={networkData}>
                <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
                <XAxis dataKey="timestamp" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} label={{ value: 'MB', angle: -90, position: 'insideLeft' }} />
                <Tooltip contentStyle={{ backgroundColor: 'rgba(26,26,26,0.95)', border: '1px solid rgba(99,102,241,0.3)', borderRadius: '8px', color: '#f5f5f5' }} />
                <Legend wrapperStyle={{ color: '#9ca3af' }} />
                <Line
                  type="monotone"
                  dataKey="rx"
                  stroke="#3b82f6"
                  name="RX (Download)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={true}
                  animationDuration={800}
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
                  animationDuration={800}
                  animationEasing="ease-in-out"
                />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <p className="text-muted-foreground">No data available</p>
          )}
        </div>
      </div>
    </div>
  );
}

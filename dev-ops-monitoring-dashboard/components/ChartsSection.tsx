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
  TooltipProps,
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

        // First, fetch one metric to determine how many CPU cores exist
        const sampleMetric = await getHistoricalMetrics(agentId, 'cpu_usage', 1);
        
        // Check for cpu_core_N metrics to determine core count
        let detectedCores = 0;
        for (let i = 0; i < 128; i++) { // Check up to 128 cores
          try {
            const coreMetric = await getHistoricalMetrics(agentId, `cpu_core_${i}`, 1);
            if (coreMetric.data_points.length > 0) {
              detectedCores = i + 1;
            } else {
              break;
            }
          } catch {
            break;
          }
        }
        setCoreCount(detectedCores);

        // Fetch all metric data in parallel
        const fetchPromises = [
          getHistoricalMetrics(agentId, 'cpu_usage', limit),
          getHistoricalMetrics(agentId, 'memory_usage', limit),
          getHistoricalMetrics(agentId, 'swap_usage', limit),
          getHistoricalMetrics(agentId, 'disk_usage', limit),
          getHistoricalMetrics(agentId, 'network_rx_bytes', limit),
          getHistoricalMetrics(agentId, 'network_tx_bytes', limit),
        ];

        // Add per-core CPU metrics
        const corePromises = [];
        for (let i = 0; i < detectedCores; i++) {
          corePromises.push(getHistoricalMetrics(agentId, `cpu_core_${i}`, limit));
        }

        const [cpuRes, memoryRes, swapRes, diskRes, networkRxRes, networkTxRes, ...coreResults] = 
          await Promise.all([...fetchPromises, ...corePromises]);

        // Process CPU data - use exact values without rounding
        const cpuChartData = cpuRes.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          cpu: point.value,
        }));
        setCpuData(cpuChartData);

        // Process per-core CPU data - use exact values
        if (coreResults.length > 0 && coreResults[0].data_points.length > 0) {
          const coreChartData = coreResults[0].data_points.map((point: DataPoint, idx: number) => {
            const dataPoint: ChartData = {
              timestamp: formatChartTime(point.timestamp),
            };
            
            // Add each core's data with full precision
            coreResults.forEach((coreRes, coreIdx) => {
              if (coreRes.data_points[idx]) {
                dataPoint[`core${coreIdx}`] = coreRes.data_points[idx].value;
              }
            });
            
            return dataPoint;
          });
          setCpuCoreData(coreChartData);
        }

        // Process Memory data (percentage values) - use exact values
        const memoryChartData = memoryRes.data_points.map((point: DataPoint, idx: number) => ({
          timestamp: formatChartTime(point.timestamp),
          memory: point.value,
          swap: swapRes.data_points[idx] ? swapRes.data_points[idx].value : 0,
        }));
        setMemoryData(memoryChartData);

        // Process Disk data (percentage values) - use exact values
        const diskChartData = diskRes.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          usage: point.value,
        }));
        setDiskData(diskChartData);

        // Process Network data - convert to MB without premature rounding
        const networkChartData = networkRxRes.data_points.map((point: DataPoint, idx: number) => ({
          timestamp: formatChartTime(point.timestamp),
          rx: point.value / (1024 * 1024),
          tx: (networkTxRes.data_points[idx]?.value / (1024 * 1024)) || 0,
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
    
    // Auto-refresh every 1 second for real-time monitoring
    const interval = setInterval(fetchChartData, 1000);
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
        {/* Per-Core CPU Chart - Full Width */}
        {!loading && cpuCoreData.length > 0 && coreCount > 0 && (
          <div className="lg:col-span-2 glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth animate-slide-up">
            <h3 className="text-lg font-semibold text-foreground mb-4">
              Per-Core CPU Usage Over Time ({coreCount} cores)
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
                      strokeWidth={2}
                      isAnimationActive={false}
                    />
                  );
                })}
              </LineChart>
            </ResponsiveContainer>
          </div>
        )}

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
                <Tooltip content={<CustomTooltip />} />
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
          <h3 className="text-lg font-semibold text-foreground mb-4">Memory & Swap Usage Over Time</h3>
          {loading ? (
            skeletonLoader
          ) : memoryData.length > 0 ? (
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
                  isAnimationActive={false}
                />
                <Area
                  type="monotone"
                  dataKey="swap"
                  name="Swap"
                  stroke="#8b5cf6"
                  fill="#8b5cf6"
                  fillOpacity={0.3}
                  isAnimationActive={false}
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
                <Tooltip content={<CustomTooltip />} />
                <Area
                  type="monotone"
                  dataKey="usage"
                  stroke="#f59e0b"
                  fill="#f59e0b"
                  fillOpacity={0.3}
                  isAnimationActive={false}
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
                <Tooltip content={<CustomTooltip />} />
                <Legend wrapperStyle={{ color: '#9ca3af' }} />
                <Line
                  type="monotone"
                  dataKey="rx"
                  stroke="#3b82f6"
                  name="RX (Download)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={false}
                />
                <Line
                  type="monotone"
                  dataKey="tx"
                  stroke="#10b981"
                  name="TX (Upload)"
                  dot={false}
                  strokeWidth={2}
                  isAnimationActive={false}
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

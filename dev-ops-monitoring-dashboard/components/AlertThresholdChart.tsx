'use client';

import { useEffect, useState } from 'react';
import {
  ComposedChart,
  Area,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  ReferenceLine,
  ReferenceArea,
} from 'recharts';
import { getHistoricalMetrics } from '@/lib/api';
import { formatChartTime } from '@/lib/metrics-utils';
import { DataPoint } from '@/types';
import { AlertTriangle, CheckCircle, XCircle } from 'lucide-react';

interface AlertThresholdChartProps {
  agentId: string;
  metric: 'cpu_usage' | 'memory_usage' | 'disk_usage';
  title: string;
  warningThreshold?: number;
  criticalThreshold?: number;
  limit?: number;
}

export default function AlertThresholdChart({
  agentId,
  metric,
  title,
  warningThreshold = 70,
  criticalThreshold = 90,
  limit = 100,
}: AlertThresholdChartProps) {
  const [data, setData] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentValue, setCurrentValue] = useState<number>(0);
  const [alertStatus, setAlertStatus] = useState<'ok' | 'warning' | 'critical'>('ok');

  useEffect(() => {
    let isInitialLoad = true;

    const fetchData = async () => {
      try {
        // Only show loading on initial mount
        if (isInitialLoad) {
          setLoading(true);
        }
        
        const result = await getHistoricalMetrics(agentId, metric, limit);
        
        const chartData = result.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          value: Number(point.value.toFixed(2)),
        }));

        setData(chartData);

        // Get current value and alert status
        if (chartData.length > 0) {
          const latest = chartData[chartData.length - 1].value;
          setCurrentValue(latest);
          
          if (latest >= criticalThreshold) {
            setAlertStatus('critical');
          } else if (latest >= warningThreshold) {
            setAlertStatus('warning');
          } else {
            setAlertStatus('ok');
          }
        }
      } catch (error) {
        console.error(`Error fetching ${metric}:`, error);
      } finally {
        if (isInitialLoad) {
          setLoading(false);
          isInitialLoad = false;
        }
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, [agentId, metric, criticalThreshold, warningThreshold, limit]);

  const getStatusColor = () => {
    switch (alertStatus) {
      case 'critical':
        return 'text-red-400';
      case 'warning':
        return 'text-amber-400';
      default:
        return 'text-green-400';
    }
  };

  const getStatusIcon = () => {
    switch (alertStatus) {
      case 'critical':
        return <XCircle className="w-5 h-5 text-red-400" />;
      case 'warning':
        return <AlertTriangle className="w-5 h-5 text-amber-400" />;
      default:
        return <CheckCircle className="w-5 h-5 text-green-400" />;
    }
  };

  const getStatusText = () => {
    switch (alertStatus) {
      case 'critical':
        return 'Critical';
      case 'warning':
        return 'Warning';
      default:
        return 'Normal';
    }
  };

  if (loading) {
    return (
      <div className="glass-morphism rounded-xl border border-border p-6">
        <div className="animate-pulse">
          <div className="h-6 bg-background/50 rounded w-1/3 mb-4"></div>
          <div className="h-64 bg-background/50 rounded"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-foreground">{title}</h3>
        <div className="flex items-center gap-2">
          {getStatusIcon()}
          <div className="text-right">
            <div className={`text-2xl font-bold ${getStatusColor()}`}>
              {currentValue.toFixed(1)}%
            </div>
            <div className="text-xs text-muted-foreground">{getStatusText()}</div>
          </div>
        </div>
      </div>

      {/* Threshold Legend */}
      <div className="flex items-center gap-4 mb-4 text-xs text-muted-foreground">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-green-500/30"></div>
          <span>Normal (&lt;{warningThreshold}%)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-amber-500/30"></div>
          <span>Warning ({warningThreshold}-{criticalThreshold}%)</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-red-500/30"></div>
          <span>Critical (&gt;{criticalThreshold}%)</span>
        </div>
      </div>

      <ResponsiveContainer width="100%" height={300}>
        <ComposedChart data={data}>
          <defs>
            <linearGradient id={`gradient-${metric}`} x1="0" y1="0" x2="0" y2="1">
              <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.8} />
              <stop offset="95%" stopColor="#3b82f6" stopOpacity={0.1} />
            </linearGradient>
          </defs>

          <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
          
          <XAxis
            dataKey="timestamp"
            stroke="#9ca3af"
            style={{ fontSize: '12px' }}
          />
          
          <YAxis
            stroke="#9ca3af"
            style={{ fontSize: '12px' }}
            domain={[0, 100]}
            label={{ value: '%', angle: -90, position: 'insideLeft' }}
          />

          {/* Critical Zone */}
          <ReferenceArea
            y1={criticalThreshold}
            y2={100}
            fill="#ef4444"
            fillOpacity={0.1}
            stroke="none"
          />

          {/* Warning Zone */}
          <ReferenceArea
            y1={warningThreshold}
            y2={criticalThreshold}
            fill="#f59e0b"
            fillOpacity={0.1}
            stroke="none"
          />

          {/* Normal Zone */}
          <ReferenceArea
            y1={0}
            y2={warningThreshold}
            fill="#10b981"
            fillOpacity={0.05}
            stroke="none"
          />

          {/* Threshold Lines */}
          <ReferenceLine
            y={warningThreshold}
            stroke="#f59e0b"
            strokeDasharray="5 5"
            strokeWidth={2}
            label={{
              value: `Warning (${warningThreshold}%)`,
              position: 'right',
              fill: '#f59e0b',
              fontSize: 12,
            }}
          />
          
          <ReferenceLine
            y={criticalThreshold}
            stroke="#ef4444"
            strokeDasharray="5 5"
            strokeWidth={2}
            label={{
              value: `Critical (${criticalThreshold}%)`,
              position: 'right',
              fill: '#ef4444',
              fontSize: 12,
            }}
          />

          <Tooltip
            contentStyle={{
              backgroundColor: 'rgba(26,26,26,0.95)',
              border: '1px solid rgba(99,102,241,0.3)',
              borderRadius: '8px',
              color: '#f5f5f5',
            }}
            formatter={(value: number) => [`${value.toFixed(2)}%`, 'Value']}
          />

          <Area
            type="monotone"
            dataKey="value"
            fill={`url(#gradient-${metric})`}
            stroke="none"
            isAnimationActive={true}
            animationDuration={800}
            animationEasing="ease-in-out"
          />

          <Line
            type="monotone"
            dataKey="value"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={false}
            isAnimationActive={true}
            animationDuration={800}
            animationEasing="ease-in-out"
          />
        </ComposedChart>
      </ResponsiveContainer>

      {/* Alert Messages */}
      {alertStatus !== 'ok' && (
        <div
          className={`mt-4 p-3 rounded-lg border ${
            alertStatus === 'critical'
              ? 'bg-red-500/10 border-red-500/30'
              : 'bg-amber-500/10 border-amber-500/30'
          }`}
        >
          <p className={`text-sm ${alertStatus === 'critical' ? 'text-red-300' : 'text-amber-300'}`}>
            {alertStatus === 'critical'
              ? `⚠️ Critical: ${metric.replace('_', ' ')} has exceeded ${criticalThreshold}%`
              : `⚠️ Warning: ${metric.replace('_', ' ')} has exceeded ${warningThreshold}%`}
          </p>
        </div>
      )}
    </div>
  );
}

'use client';

import Link from 'next/link';
import { HardDrive, Zap, Database } from 'lucide-react';
import StatusBadge from './StatusBadge';
import { formatRelativeTime, formatBytes, calculatePercentage, extractMetric } from '@/lib/metrics-utils';
import { Agent } from '@/types';
import { useMetricsContext } from '@/contexts/MetricsContext';

interface ServerCardProps {
  agent: Agent;
}

export default function ServerCard({ agent }: ServerCardProps) {
  // Get metrics from shared context
  const { agentMetrics, initialStateReceived } = useMetricsContext();
  const metrics = agentMetrics[agent.id] || null;
  const loading = !initialStateReceived;

  const borderColorMap = {
    Healthy: 'border-green-700/50 hover:border-green-600',
    Degraded: 'border-yellow-700/50 hover:border-yellow-600',
    Unreachable: 'border-red-700/50 hover:border-red-600',
  };

  const bgColorMap = {
    Healthy: 'bg-green-900/5 hover:bg-green-900/10',
    Degraded: 'bg-yellow-900/5 hover:bg-yellow-900/10',
    Unreachable: 'bg-red-900/5 hover:bg-red-900/10',
  };

  let cpuPercent = 0;
  let memoryPercent = 0;
  let diskPercent = 0;
  let memoryDisplay = '-';
  let diskDisplay = '-';

  if (metrics) {
    cpuPercent = extractMetric(metrics.metrics, 'cpu_usage');
    memoryPercent = extractMetric(metrics.metrics, 'memory_usage');

    // Memory display - get from metrics
    const memoryUsed = extractMetric(metrics.metrics, 'memory_used_bytes');
    const memoryTotal = extractMetric(metrics.metrics, 'memory_total_bytes');
    if (memoryTotal > 0) {
      memoryDisplay = `${formatBytes(memoryUsed)} / ${formatBytes(memoryTotal)}`;
    }

    // Disk display - get from metrics
    const diskUsed = extractMetric(metrics.metrics, 'disk_used_bytes');
    const diskTotal = extractMetric(metrics.metrics, 'disk_total_bytes');
    if (diskTotal > 0) {
      diskPercent = calculatePercentage(diskUsed, diskTotal);
      diskDisplay = `${formatBytes(diskUsed)} / ${formatBytes(diskTotal)}`;
    } else {
      // Fallback to disk_usage percentage if bytes not available
      diskPercent = extractMetric(metrics.metrics, 'disk_usage');
    }
  }

  return (
    <Link href={`/server/${encodeURIComponent(agent.id)}`}>
      <div
        className={`block glass-morphism rounded-xl p-6 transition-smooth cursor-pointer group hover:shadow-xl ${borderColorMap[agent.status]} ${bgColorMap[agent.status]}`}
      >
        <div className="flex items-start justify-between mb-4">
          <div className="flex-1">
            <h3 className="text-lg font-bold text-foreground group-hover:text-primary transition-colors">{agent.name}</h3>
            <p className="text-xs text-muted-foreground mt-1">Last seen: {formatRelativeTime(agent.last_seen)}</p>
          </div>
          <StatusBadge status={agent.status} size="sm" />
        </div>

        <div className="space-y-4">
          <div className="grid grid-cols-3 gap-3">
            <div>
              <div className="flex items-center gap-1 mb-2">
                <Zap className="w-4 h-4 text-blue-400" />
                <span className="text-xs text-muted-foreground">CPU</span>
              </div>
              <p className="text-lg font-semibold text-white">
                {loading ? '-' : cpuPercent.toFixed(1)}%
              </p>
              {!loading && (
                <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full transition-all ${
                      cpuPercent < 70 ? 'bg-gradient-to-r from-emerald-400 to-emerald-500' : cpuPercent < 85 ? 'bg-gradient-to-r from-amber-400 to-amber-500' : 'bg-gradient-to-r from-red-400 to-red-500'
                    }`}
                    style={{ width: `${Math.min(cpuPercent, 100)}%` }}
                  />
                </div>
              )}
            </div>

            <div>
              <div className="flex items-center gap-1 mb-2">
                <Database className="w-4 h-4 text-purple-400" />
                <span className="text-xs text-muted-foreground">Memory</span>
              </div>
              <p className="text-lg font-semibold text-white">
                {loading ? '-' : memoryPercent.toFixed(1)}%
              </p>
              {!loading && (
                <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full transition-all ${
                      memoryPercent < 80 ? 'bg-gradient-to-r from-emerald-400 to-emerald-500' : memoryPercent < 90 ? 'bg-gradient-to-r from-amber-400 to-amber-500' : 'bg-gradient-to-r from-red-400 to-red-500'
                    }`}
                    style={{ width: `${Math.min(memoryPercent, 100)}%` }}
                  />
                </div>
              )}
            </div>

            <div>
              <div className="flex items-center gap-1 mb-2">
                <HardDrive className="w-4 h-4 text-orange-400" />
                <span className="text-xs text-muted-foreground">Disk</span>
              </div>
              <p className="text-lg font-semibold text-white">
                {loading ? '-' : diskPercent.toFixed(1)}%
              </p>
              {!loading && (
                <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                  <div
                    className={`h-full rounded-full transition-all ${
                      diskPercent < 70 ? 'bg-gradient-to-r from-emerald-400 to-emerald-500' : diskPercent < 90 ? 'bg-gradient-to-r from-amber-400 to-amber-500' : 'bg-gradient-to-r from-red-400 to-red-500'
                    }`}
                    style={{ width: `${Math.min(diskPercent, 100)}%` }}
                  />
                </div>
              )}
            </div>
          </div>

          {!loading && (
            <div className="pt-3 border-t border-border text-xs text-muted-foreground space-y-1">
              <p>Memory: {memoryDisplay}</p>
              <p>Disk: {diskDisplay}</p>
            </div>
          )}

          <div className="pt-3">
            <span className="text-xs font-medium text-accent group-hover:text-primary transition-colors">
              View Details →
            </span>
          </div>
        </div>
      </div>
    </Link>
  );
}

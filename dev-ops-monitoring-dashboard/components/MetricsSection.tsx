'use client';

import { Activity, Database, HardDrive, Wifi } from 'lucide-react';
import MetricCard from './MetricCard';
import { LatestMetrics } from '@/types';
import { formatBytes, calculatePercentage, extractMetric, getMetricColor } from '@/lib/metrics-utils';

interface MetricsSectionProps {
  metrics: LatestMetrics | null;
  loading: boolean;
}

export default function MetricsSection({ metrics, loading }: MetricsSectionProps) {
  let cpuPercent = 0;
  let memUsed = 0;
  let memTotal = 0;
  let memPercent = 0;
  let diskUsed = 0;
  let diskTotal = 0;
  let diskPercent = 0;
  let networkRx = 0;
  let networkTx = 0;

  if (metrics) {
    cpuPercent = extractMetric(metrics.metrics, 'cpu_percent');

    memUsed = extractMetric(metrics.metrics, 'memory_used_bytes');
    memTotal = extractMetric(metrics.metrics, 'memory_total_bytes');
    memPercent = calculatePercentage(memUsed, memTotal);

    diskUsed = extractMetric(metrics.metrics, 'disk_used_bytes');
    diskTotal = extractMetric(metrics.metrics, 'disk_total_bytes');
    diskPercent = calculatePercentage(diskUsed, diskTotal);

    networkRx = extractMetric(metrics.metrics, 'network_rx_bytes');
    networkTx = extractMetric(metrics.metrics, 'network_tx_bytes');
  }

  const cpuColor = getMetricColor('cpu_percent', cpuPercent);
  const memColor = getMetricColor('memory', memPercent);
  const diskColor = getMetricColor('disk', diskPercent);

  const skeletonLoader = (
    <div className="h-32 bg-background/30 rounded-xl animate-pulse" />
  );

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8 animate-slide-up">
      {loading ? (
        <>
          {skeletonLoader}
          {skeletonLoader}
          {skeletonLoader}
          {skeletonLoader}
        </>
      ) : (
        <>
          <MetricCard
            label="CPU Usage"
            value={cpuPercent.toFixed(1)}
            unit="%"
            percentage={cpuPercent}
            color={cpuColor}
            icon={<Activity className="w-6 h-6" />}
            size="md"
          />

          <MetricCard
            label="Memory Usage"
            value={formatBytes(memUsed)}
            unit={`/ ${formatBytes(memTotal)}`}
            percentage={memPercent}
            color={memColor}
            icon={<Database className="w-6 h-6" />}
            secondaryValue={`${memPercent.toFixed(1)}% used`}
            size="md"
          />

          <MetricCard
            label="Disk Usage"
            value={formatBytes(diskUsed)}
            unit={`/ ${formatBytes(diskTotal)}`}
            percentage={diskPercent}
            color={diskColor}
            icon={<HardDrive className="w-6 h-6" />}
            secondaryValue={`${diskPercent.toFixed(1)}% used`}
            size="md"
          />

          <MetricCard
            label="Network Traffic"
            value={formatBytes(networkRx)}
            unit="RX"
            color="blue"
            icon={<Wifi className="w-6 h-6" />}
            secondaryValue={`TX: ${formatBytes(networkTx)}`}
            size="md"
          />
        </>
      )}
    </div>
  );
}

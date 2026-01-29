'use client';

import { memo } from 'react';
import { Activity, Database, HardDrive, Wifi, Cpu, Thermometer, MemoryStick } from 'lucide-react';
import MetricCard from './MetricCard';
import { LatestMetrics } from '@/types';
import { formatBytes, calculatePercentage, extractMetric, getMetricColor } from '@/lib/metrics-utils';

interface MetricsSectionProps {
  metrics: LatestMetrics | null;
  loading: boolean;
}

function MetricsSection({ metrics, loading }: MetricsSectionProps) {
  let cpuPercent = 0;
  let memUsed = 0;
  let memTotal = 0;
  let memPercent = 0;
  let swapPercent = 0;
  let swapUsed = 0;
  let swapTotal = 0;
  let diskUsed = 0;
  let diskTotal = 0;
  let diskPercent = 0;
  let networkRx = 0;
  let networkTx = 0;
  let cpuTemp: number | null = null;
  let gpuTemp: number | null = null;
  let gpuUsage: number | null = null;
  let gpuMemUsed: number | null = null;
  let gpuMemTotal: number | null = null;
  
  // Extract per-core CPU metrics
  let cpuCores: number[] = [];

  if (metrics) {
    // Agent sends percentage values directly
    cpuPercent = extractMetric(metrics.metrics, 'cpu_usage');
    memPercent = extractMetric(metrics.metrics, 'memory_usage');
    diskPercent = extractMetric(metrics.metrics, 'disk_usage');
    swapPercent = extractMetric(metrics.metrics, 'swap_usage');

    // Extract disk space in bytes
    diskUsed = extractMetric(metrics.metrics, 'disk_used_bytes');
    diskTotal = extractMetric(metrics.metrics, 'disk_total_bytes');

    // Extract swap in bytes
    swapUsed = extractMetric(metrics.metrics, 'swap_used_bytes');
    swapTotal = extractMetric(metrics.metrics, 'swap_total_bytes');

    // For display, we'll show percentages for memory since agent only sends percentage
    memUsed = memPercent;
    memTotal = 100;

    networkRx = extractMetric(metrics.metrics, 'network_rx_bytes');
    networkTx = extractMetric(metrics.metrics, 'network_tx_bytes');
    
    // Temperature metrics
    const cpuTempRaw = extractMetric(metrics.metrics, 'cpu_temp_celsius');
    const gpuTempRaw = extractMetric(metrics.metrics, 'gpu_temp_celsius');
    cpuTemp = cpuTempRaw > 0 ? cpuTempRaw : null;
    gpuTemp = gpuTempRaw > 0 ? gpuTempRaw : null;
    
    // GPU metrics
    const gpuUsageRaw = extractMetric(metrics.metrics, 'gpu_usage');
    const gpuMemUsedRaw = extractMetric(metrics.metrics, 'gpu_memory_used_bytes');
    const gpuMemTotalRaw = extractMetric(metrics.metrics, 'gpu_memory_total_bytes');
    gpuUsage = gpuUsageRaw > 0 ? gpuUsageRaw : null;
    gpuMemUsed = gpuMemUsedRaw > 0 ? gpuMemUsedRaw : null;
    gpuMemTotal = gpuMemTotalRaw > 0 ? gpuMemTotalRaw : null;
    
    // Extract per-core CPU metrics
    let coreIndex = 0;
    while (true) {
      const coreUsage = extractMetric(metrics.metrics, `cpu_core_${coreIndex}`);
      if (coreUsage === 0 && coreIndex > 0) break; // Stop when we don't find more cores
      if (coreUsage >= 0) {
        cpuCores.push(coreUsage);
        coreIndex++;
      } else {
        break;
      }
    }
  }

  const cpuColor = getMetricColor('cpu_percent', cpuPercent);
  const memColor = getMetricColor('memory', memPercent);
  const diskColor = getMetricColor('disk', diskPercent);
  const swapColor = getMetricColor('memory', swapPercent);
  const gpuColor = gpuUsage ? getMetricColor('cpu_percent', gpuUsage) : 'gray';
  
  const getTempColor = (temp: number | null): string => {
    if (!temp) return 'gray';
    if (temp >= 80) return 'red';
    if (temp >= 70) return 'amber';
    if (temp >= 60) return 'yellow';
    return 'emerald';
  };

  const skeletonLoader = (
    <div className="h-32 bg-background/30 rounded-xl animate-pulse" />
  );

  return (
    <>
      {/* Main Metrics Grid */}
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
              value={cpuPercent.toFixed(2)}
              unit="%"
              percentage={cpuPercent}
              color={cpuColor}
              icon={<Activity className="w-6 h-6" />}
              secondaryValue={cpuCores.length > 0 ? `${cpuCores.length} cores` : undefined}
              size="md"
            />

            <MetricCard
              label="Memory Usage"
              value={memPercent.toFixed(2)}
              unit="%"
              percentage={memPercent}
              color={memColor}
              icon={<Database className="w-6 h-6" />}
              secondaryValue={memPercent < 75 ? 'Healthy' : memPercent < 90 ? 'Warning' : 'Critical'}
              size="md"
            />

            <MetricCard
              label="Disk Usage"
              value={formatBytes(diskUsed)}
              unit={`/ ${formatBytes(diskTotal)}`}
              percentage={diskPercent}
              color={diskColor}
              icon={<HardDrive className="w-6 h-6" />}
              secondaryValue={`${diskPercent.toFixed(2)}% used`}
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

      {/* Extended Metrics Grid - Swap, Temperature, GPU */}
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
              label="Swap Memory"
              value={swapPercent.toFixed(2)}
              unit="%"
              percentage={swapPercent}
              color={swapColor}
              icon={<MemoryStick className="w-6 h-6" />}
              secondaryValue={swapTotal > 0 ? `${formatBytes(swapUsed)} / ${formatBytes(swapTotal)}` : 'No swap'}
              size="md"
            />

            <MetricCard
              label="CPU Temperature"
              value={cpuTemp ? cpuTemp.toFixed(1) : 'N/A'}
              unit={cpuTemp ? '°C' : ''}
              percentage={cpuTemp ? (cpuTemp / 100) * 100 : 0}
              color={cpuTemp && cpuTemp >= 80 ? 'red' : cpuTemp && cpuTemp >= 70 ? 'yellow' : cpuTemp ? 'green' : undefined}
              icon={<Thermometer className="w-6 h-6" />}
              secondaryValue={cpuTemp ? (cpuTemp < 70 ? 'Normal' : cpuTemp < 80 ? 'Warm' : 'Hot') : 'Not available'}
              size="md"
            />

            <MetricCard
              label="GPU Temperature"
              value={gpuTemp ? gpuTemp.toFixed(1) : 'N/A'}
              unit={gpuTemp ? '°C' : ''}
              percentage={gpuTemp ? (gpuTemp / 100) * 100 : 0}
              color={gpuTemp && gpuTemp >= 80 ? 'red' : gpuTemp && gpuTemp >= 70 ? 'yellow' : gpuTemp ? 'green' : undefined}
              icon={<Thermometer className="w-6 h-6" />}
              secondaryValue={gpuTemp ? (gpuTemp < 70 ? 'Normal' : gpuTemp < 80 ? 'Warm' : 'Hot') : 'Not available'}
              size="md"
            />

            <MetricCard
              label="GPU Usage"
              value={gpuUsage ? gpuUsage.toFixed(1) : 'N/A'}
              unit={gpuUsage ? '%' : ''}
              percentage={gpuUsage || 0}
              color={gpuUsage ? getMetricColor('cpu_percent', gpuUsage) : undefined}
              icon={<Cpu className="w-6 h-6" />}
              secondaryValue={gpuUsage ? (gpuMemUsed && gpuMemTotal ? `${formatBytes(gpuMemUsed)} / ${formatBytes(gpuMemTotal)}` : 'Memory N/A') : 'Not available'}
              size="md"
            />
          </>
        )}
      </div>

      {/* Per-Core CPU Breakdown */}
    </>
  );
}

export default memo(MetricsSection);

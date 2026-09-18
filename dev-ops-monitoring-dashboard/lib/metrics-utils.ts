export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KiB', 'MiB', 'GiB', 'TiB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

export function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const date = new Date(timestamp);
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return `${seconds} second${seconds !== 1 ? 's' : ''} ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minute${minutes !== 1 ? 's' : ''} ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hour${hours !== 1 ? 's' : ''} ago`;
  const days = Math.floor(hours / 24);
  return `${days} day${days !== 1 ? 's' : ''} ago`;
}

export function getStatusColor(status: string): 'green' | 'yellow' | 'red' | 'gray' {
  if (status === 'Healthy') return 'green';
  if (status === 'Degraded') return 'yellow';
  if (status === 'Unreachable') return 'red';
  return 'gray';
}

export function getMetricColor(metric: string, percentage: number): 'green' | 'yellow' | 'red' | 'blue' {
  if (metric === 'cpu_percent') {
    if (percentage < 70) return 'green';
    if (percentage < 85) return 'yellow';
    return 'red';
  }
  if (metric === 'memory' || metric === 'disk') {
    if (percentage < 80) return 'green';
    if (percentage < 90) return 'yellow';
    return 'red';
  }
  return 'blue';
}

export function formatChartTime(timestamp: string): string {
  const date = new Date(timestamp);
  // Show HH:MM:SS for real-time precision
  return date.toLocaleTimeString('en-US', { 
    hour: '2-digit', 
    minute: '2-digit', 
    second: '2-digit',
    hour12: false // Use 24-hour format for cleaner display
  });
}

export function extractMetric(metrics: any[], metricName: string): number {
  const metric = metrics.find(m => m.name === metricName);
  return metric?.value ?? 0;
}

export function calculatePercentage(used: number, total: number): number {
  if (total === 0) return 0;
  return Math.round((used / total) * 100 * 10) / 10;
}

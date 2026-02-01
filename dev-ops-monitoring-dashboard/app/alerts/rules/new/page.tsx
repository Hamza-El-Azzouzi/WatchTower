'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import Link from 'next/link';
import { ArrowLeft, Plus, AlertTriangle } from 'lucide-react';
import { createAlertRule } from '@/lib/alerts-api';

const COMMON_METRICS = [
  { value: 'cpu_usage', label: 'CPU Usage (%)' },
  { value: 'memory_usage', label: 'Memory Usage (%)' },
  { value: 'disk_usage', label: 'Disk Usage (%)' },
  { value: 'network_rx_bytes', label: 'Network RX (bytes)' },
  { value: 'network_tx_bytes', label: 'Network TX (bytes)' },
  { value: 'db_connections_active', label: 'DB Active Connections' },
  { value: 'db_cache_hit_ratio', label: 'DB Cache Hit Ratio (%)' },
  { value: 'db_slow_queries', label: 'DB Slow Queries' },
  { value: 'db_locks_waiting', label: 'DB Waiting Locks' },
];

export default function NewAlertRulePage() {
  const router = useRouter();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    metric: 'cpu_usage',
    condition: 'greater_than' as const,
    threshold: 80,
    duration_seconds: 300,
    severity: 'warning' as 'info' | 'warning' | 'critical',
    channels: [] as string[],
    cooldown_seconds: 300,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);

    try {
      await createAlertRule(formData);
      router.push('/alerts');
    } catch (err) {
      setError('Failed to create alert rule. Please try again.');
      setLoading(false);
    }
  };

  return (
    <div className="p-8 max-w-4xl mx-auto">
      <Link href="/alerts" className="inline-flex items-center gap-2 text-primary hover:underline mb-6">
        <ArrowLeft className="w-4 h-4" />
        Back to Alerts
      </Link>

      <div className="mb-6">
        <div className="flex items-center gap-3 mb-2">
          <Plus className="w-8 h-8 text-primary" />
          <h1 className="text-3xl font-bold">Create Alert Rule</h1>
        </div>
        <p className="text-muted-foreground">Define conditions to monitor your infrastructure</p>
      </div>

      {error && (
        <div className="glass-morphism rounded-xl p-4 mb-6 border border-red-500/30 bg-red-500/5">
          <div className="flex items-center gap-2 text-red-400">
            <AlertTriangle className="w-5 h-5" />
            <span>{error}</span>
          </div>
        </div>
      )}

      <form onSubmit={handleSubmit} className="glass-morphism rounded-xl p-8 space-y-6">
        {/* Basic Info */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Basic Information</h2>
          
          <div>
            <label className="block text-sm font-medium mb-2">
              Alert Name <span className="text-red-400">*</span>
            </label>
            <input
              type="text"
              required
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              placeholder="e.g., High CPU Usage"
            />
          </div>

          <div>
            <label className="block text-sm font-medium mb-2">
              Description
            </label>
            <textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              placeholder="Optional description of what this alert monitors"
              rows={3}
            />
          </div>
        </div>

        {/* Condition */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Alert Condition</h2>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-2">
                Metric <span className="text-red-400">*</span>
              </label>
              <select
                value={formData.metric}
                onChange={(e) => setFormData({ ...formData, metric: e.target.value })}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              >
                {COMMON_METRICS.map((metric) => (
                  <option key={metric.value} value={metric.value}>
                    {metric.label}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                Condition <span className="text-red-400">*</span>
              </label>
              <select
                value={formData.condition}
                onChange={(e) => setFormData({ ...formData, condition: e.target.value as any })}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              >
                <option value="greater_than">Greater Than (&gt;)</option>
                <option value="less_than">Less Than (&lt;)</option>
                <option value="equals">Equals (=)</option>
                <option value="not_equals">Not Equals (≠)</option>
              </select>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium mb-2">
                Threshold <span className="text-red-400">*</span>
              </label>
              <input
                type="number"
                required
                step="0.01"
                value={formData.threshold}
                onChange={(e) => setFormData({ ...formData, threshold: parseFloat(e.target.value) })}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                Duration (seconds) <span className="text-red-400">*</span>
              </label>
              <input
                type="number"
                required
                min="0"
                value={formData.duration_seconds}
                onChange={(e) => setFormData({ ...formData, duration_seconds: parseInt(e.target.value) })}
                className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
              />
              <p className="text-xs text-muted-foreground mt-1">
                Alert fires only if condition is met for this duration
              </p>
            </div>
          </div>
        </div>

        {/* Severity */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Alert Severity</h2>
          
          <div className="grid grid-cols-3 gap-4">
            <button
              type="button"
              onClick={() => setFormData({ ...formData, severity: 'info' })}
              className={`p-4 rounded-lg border-2 transition-all ${
                formData.severity === 'info'
                  ? 'border-blue-500 bg-blue-500/10'
                  : 'border-gray-700 hover:border-blue-500/50'
              }`}
            >
              <div className="text-2xl mb-2">ℹ️</div>
              <div className="font-semibold">Info</div>
              <div className="text-xs text-muted-foreground">Low priority</div>
            </button>

            <button
              type="button"
              onClick={() => setFormData({ ...formData, severity: 'warning' })}
              className={`p-4 rounded-lg border-2 transition-all ${
                formData.severity === 'warning'
                  ? 'border-yellow-500 bg-yellow-500/10'
                  : 'border-gray-700 hover:border-yellow-500/50'
              }`}
            >
              <div className="text-2xl mb-2">⚠️</div>
              <div className="font-semibold">Warning</div>
              <div className="text-xs text-muted-foreground">Medium priority</div>
            </button>

            <button
              type="button"
              onClick={() => setFormData({ ...formData, severity: 'critical' })}
              className={`p-4 rounded-lg border-2 transition-all ${
                formData.severity === 'critical'
                  ? 'border-red-500 bg-red-500/10'
                  : 'border-gray-700 hover:border-red-500/50'
              }`}
            >
              <div className="text-2xl mb-2">🚨</div>
              <div className="font-semibold">Critical</div>
              <div className="text-xs text-muted-foreground">High priority</div>
            </button>
          </div>
        </div>

        {/* Advanced Settings */}
        <div className="space-y-4">
          <h2 className="text-xl font-semibold">Advanced Settings</h2>
          
          <div>
            <label className="block text-sm font-medium mb-2">
              Cooldown (seconds)
            </label>
            <input
              type="number"
              min="0"
              value={formData.cooldown_seconds}
              onChange={(e) => setFormData({ ...formData, cooldown_seconds: parseInt(e.target.value) })}
              className="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-primary"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Minimum time between repeated notifications for the same alert
            </p>
          </div>
        </div>

        {/* Actions */}
        <div className="flex items-center justify-end gap-4 pt-4 border-t border-gray-700">
          <Link
            href="/alerts"
            className="px-6 py-2 rounded-lg border border-gray-700 hover:bg-gray-800 transition-colors"
          >
            Cancel
          </Link>
          <button
            type="submit"
            disabled={loading}
            className="px-6 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
          >
            {loading ? 'Creating...' : 'Create Alert Rule'}
          </button>
        </div>
      </form>
    </div>
  );
}

'use client'

import { useEffect, useState } from 'react'
import { useRouter, useParams } from 'next/navigation'
import { ArrowLeft } from 'lucide-react'
import { getAlertRule, getNotificationChannels, updateAlertRule, type NotificationChannel } from '@/lib/alerts-api'

export default function EditAlertRulePage() {
  const router = useRouter()
  const params = useParams()
  const ruleId = params.id as string

  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [notificationChannels, setNotificationChannels] = useState<NotificationChannel[]>([])
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    metric: 'cpu_usage',
    condition: 'greater_than' as 'greater_than' | 'less_than' | 'equals' | 'not_equals',
    threshold: 80,
    duration: 30,
    severity: 'warning' as 'info' | 'warning' | 'critical',
    cooldown: 300,
    channels: [] as string[],
  })

  useEffect(() => {
    const fetchRule = async () => {
      try {
        const [data, channels] = await Promise.all([getAlertRule(ruleId), getNotificationChannels()])
        setNotificationChannels(channels)
        setFormData({
          name: data.name,
          description: data.description || '',
          metric: data.metric,
          condition: data.condition,
          threshold: data.threshold,
          duration: data.duration_seconds,
          severity: data.severity,
          cooldown: data.cooldown_seconds || 300,
          channels: data.channels,
        })
        setLoading(false)
      } catch (err) {
        setError('Failed to load alert rule')
        setLoading(false)
      }
    }

    if (ruleId) {
      fetchRule()
    }
  }, [ruleId])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')

    try {
      await updateAlertRule(ruleId, {
          name: formData.name,
          description: formData.description,
          metric: formData.metric,
          condition: formData.condition,
          threshold: Number(formData.threshold),
          duration_seconds: Number(formData.duration),
          severity: formData.severity,
          cooldown_seconds: Number(formData.cooldown),
          channels: formData.channels,
      })

      router.push('/alerts')
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : 'Failed to update alert rule')
    }
  }

  const metrics = [
    { value: 'cpu_usage', label: 'CPU Usage (%)' },
    { value: 'memory_usage', label: 'Memory Usage (%)' },
    { value: 'disk_usage', label: 'Disk Usage (%)' },
    { value: 'network_rx', label: 'Network RX (bytes/s)' },
    { value: 'network_tx', label: 'Network TX (bytes/s)' },
    { value: 'db_connections_active', label: 'Active DB Connections' },
    { value: 'db_cache_hit_ratio', label: 'DB Cache Hit Ratio (%)' },
    { value: 'db_slow_queries', label: 'DB Slow Queries' },
    { value: 'db_locks_waiting', label: 'DB Locks Waiting' },
  ]

  const conditions = [
    { value: 'greater_than', label: 'Greater Than (>)' },
    { value: 'less_than', label: 'Less Than (<)' },
    { value: 'equals', label: 'Equals (=)' },
    { value: 'not_equals', label: 'Not Equals (≠)' },
  ]

  if (loading) {
    return (
      <div className="min-h-screen bg-background p-8">
        <div className="max-w-3xl mx-auto">
          <div className="text-center py-12">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary mx-auto"></div>
            <p className="mt-4 text-muted-foreground">Loading alert rule...</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-3xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <button
            onClick={() => router.back()}
            className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-smooth mb-4"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Alerts
          </button>
          <h1 className="text-3xl font-bold text-foreground">Edit Alert Rule</h1>
          <p className="text-muted-foreground mt-2">
            Update the alert rule configuration
          </p>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="space-y-6">
          {error && (
            <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-500">
              {error}
            </div>
          )}

          {/* Basic Info */}
          <div className="bg-card rounded-lg p-6 border border-border space-y-4">
            <h2 className="text-lg font-semibold text-foreground">Basic Information</h2>
            
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Rule Name *
              </label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="e.g., High CPU Usage"
                required
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Description
              </label>
              <textarea
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="Optional description"
                rows={3}
              />
            </div>
          </div>

          {/* Condition */}
          <div className="bg-card rounded-lg p-6 border border-border space-y-4">
            <h2 className="text-lg font-semibold text-foreground">Alert Condition</h2>
            
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Metric *
              </label>
              <select
                value={formData.metric}
                onChange={(e) => setFormData({ ...formData, metric: e.target.value })}
                className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                required
              >
                {metrics.map((m) => (
                  <option key={m.value} value={m.value}>
                    {m.label}
                  </option>
                ))}
              </select>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-foreground mb-2">
                  Condition *
                </label>
                <select
                  value={formData.condition}
                  onChange={(e) => setFormData({ ...formData, condition: e.target.value as typeof formData.condition })}
                  className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                  required
                >
                  {conditions.map((c) => (
                    <option key={c.value} value={c.value}>
                      {c.label}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-foreground mb-2">
                  Threshold *
                </label>
                <input
                  type="number"
                  value={formData.threshold}
                  onChange={(e) => setFormData({ ...formData, threshold: Number(e.target.value) })}
                  className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                  placeholder="e.g., 80"
                  required
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Duration (seconds) *
              </label>
              <input
                type="number"
                value={formData.duration}
                onChange={(e) => setFormData({ ...formData, duration: Number(e.target.value) })}
                className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="e.g., 30"
                required
              />
              <p className="text-sm text-muted-foreground mt-1">
                How long the condition must be true before the alert fires
              </p>
            </div>
          </div>

          {/* Severity */}
          <div className="bg-card rounded-lg p-6 border border-border space-y-4">
            <h2 className="text-lg font-semibold text-foreground">Severity</h2>
            
            <div className="flex gap-3">
              {(['info', 'warning', 'critical'] as const).map((severity) => (
                <button
                  key={severity}
                  type="button"
                  onClick={() => setFormData({ ...formData, severity })}
                  className={`flex-1 py-3 px-4 rounded-lg font-medium transition-smooth ${
                    formData.severity === severity
                      ? severity === 'info'
                        ? 'bg-blue-500 text-white'
                        : severity === 'warning'
                        ? 'bg-yellow-500 text-white'
                        : 'bg-red-500 text-white'
                      : 'bg-background border border-border text-foreground hover:border-primary'
                  }`}
                >
                  {severity === 'info' && 'ℹ️'}
                  {severity === 'warning' && '⚠️'}
                  {severity === 'critical' && '🔴'}
                  {' '}
                  {severity.charAt(0).toUpperCase() + severity.slice(1)}
                </button>
              ))}
            </div>
          </div>

          {/* Advanced */}
          <div className="bg-card rounded-lg p-6 border border-border space-y-4">
            <h2 className="text-lg font-semibold text-foreground">Advanced Settings</h2>

            <div>
              <label className="block text-sm font-medium text-foreground mb-2">Notification destinations</label>
              <div className="grid gap-2 sm:grid-cols-2">
                {notificationChannels.map(channel => <label key={channel.id} className="flex items-center gap-3 rounded-lg border border-border p-3"><input type="checkbox" checked={formData.channels.includes(channel.id)} onChange={event => setFormData(previous => ({ ...previous, channels: event.target.checked ? [...previous.channels, channel.id] : previous.channels.filter(id => id !== channel.id) }))} /><span className="text-sm">{channel.name}</span></label>)}
              </div>
            </div>
            
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                Cooldown (seconds)
              </label>
              <input
                type="number"
                value={formData.cooldown}
                onChange={(e) => setFormData({ ...formData, cooldown: Number(e.target.value) })}
                className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="e.g., 300"
              />
              <p className="text-sm text-muted-foreground mt-1">
                Minimum time between repeated alerts (prevents spam)
              </p>
            </div>
          </div>

          {/* Actions */}
          <div className="flex gap-4">
            <button
              type="submit"
              className="flex-1 bg-primary text-primary-foreground px-6 py-3 rounded-lg font-medium hover:bg-primary/90 transition-smooth"
            >
              Update Alert Rule
            </button>
            <button
              type="button"
              onClick={() => router.back()}
              className="px-6 py-3 bg-background border border-border rounded-lg font-medium text-foreground hover:bg-accent transition-smooth"
            >
              Cancel
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

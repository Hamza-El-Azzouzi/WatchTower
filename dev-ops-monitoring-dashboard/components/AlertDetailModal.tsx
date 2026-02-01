'use client'

import { X, Clock, User, CheckCircle, AlertTriangle, Info } from 'lucide-react'
import { useEffect, useState } from 'react'

interface Alert {
  id: string
  rule_id: string
  rule_name: string
  agent_id: string
  agent_name: string
  state: string
  metric: string
  current_value: number
  threshold: number
  condition: string
  severity: string
  message: string
  triggered_at: string
  fired_at?: string
  resolved_at?: string
  acknowledged: boolean
  acknowledged_at?: string
  acknowledged_by?: string
  last_notification_at?: string
}

interface AlertDetailModalProps {
  alert: Alert
  onClose: () => void
  onAcknowledge?: (alertId: string) => void
}

export function AlertDetailModal({ alert, onClose, onAcknowledge }: AlertDetailModalProps) {
  const [acknowledgeNote, setAcknowledgeNote] = useState('')
  const [acknowledgeBy, setAcknowledgeBy] = useState('')
  const [showAcknowledgeForm, setShowAcknowledgeForm] = useState(false)

  useEffect(() => {
    // Prevent body scroll when modal is open
    document.body.style.overflow = 'hidden'
    return () => {
      document.body.style.overflow = 'unset'
    }
  }, [])

  const formatTimestamp = (timestamp?: string) => {
    if (!timestamp) return 'N/A'
    return new Date(timestamp).toLocaleString()
  }

  const getDuration = (start: string, end?: string) => {
    const startTime = new Date(start).getTime()
    const endTime = end ? new Date(end).getTime() : Date.now()
    const duration = Math.floor((endTime - startTime) / 1000)
    
    if (duration < 60) return `${duration}s`
    if (duration < 3600) return `${Math.floor(duration / 60)}m ${duration % 60}s`
    return `${Math.floor(duration / 3600)}h ${Math.floor((duration % 3600) / 60)}m`
  }

  const getSeverityColor = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical': return 'text-red-500'
      case 'warning': return 'text-yellow-500'
      case 'info': return 'text-blue-500'
      default: return 'text-gray-500'
    }
  }

  const getSeverityBg = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical': return 'bg-red-500/10 border-red-500/20'
      case 'warning': return 'bg-yellow-500/10 border-yellow-500/20'
      case 'info': return 'bg-blue-500/10 border-blue-500/20'
      default: return 'bg-gray-500/10 border-gray-500/20'
    }
  }

  const getStateColor = (state: string) => {
    switch (state.toLowerCase()) {
      case 'firing': return 'bg-red-500'
      case 'pending': return 'bg-yellow-500'
      case 'resolved': return 'bg-green-500'
      default: return 'bg-gray-500'
    }
  }

  const handleAcknowledge = async () => {
    if (!acknowledgeBy.trim()) {
      window.alert('Please enter your name')
      return
    }

    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'
      const response = await fetch(`${apiUrl}/api/v1/alerts/${alert.id}/acknowledge`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          acknowledged_by: acknowledgeBy,
          note: acknowledgeNote || undefined,
        }),
      })

      if (!response.ok) throw new Error('Failed to acknowledge alert')

      if (onAcknowledge) {
        onAcknowledge(alert.id)
      }
      
      setShowAcknowledgeForm(false)
      // Refresh or close modal
      setTimeout(() => {
        window.location.reload()
      }, 500)
    } catch (err) {
      console.error('Failed to acknowledge alert:', err)
      window.alert('Failed to acknowledge alert')
    }
  }

  return (
    <div className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div className="bg-card border border-border rounded-lg shadow-2xl max-w-3xl w-full max-h-[90vh] overflow-y-auto">
        {/* Header */}
        <div className="sticky top-0 bg-card border-b border-border p-6 flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-3 mb-2">
              <span className={`px-3 py-1 rounded-full text-xs font-bold ${getStateColor(alert.state)} text-white`}>
                {alert.state.toUpperCase()}
              </span>
              <span className={`px-3 py-1 rounded-full text-xs font-bold ${getSeverityBg(alert.severity)} ${getSeverityColor(alert.severity)}`}>
                {alert.severity.toUpperCase()}
              </span>
            </div>
            <h2 className="text-2xl font-bold text-foreground">{alert.rule_name}</h2>
            <p className="text-muted-foreground mt-1">{alert.message}</p>
          </div>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-smooth"
          >
            <X className="w-6 h-6" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 space-y-6">
          {/* Alert Status */}
          <div className="grid grid-cols-2 gap-4">
            <div className="bg-background rounded-lg p-4 border border-border">
              <div className="text-sm text-muted-foreground mb-1">Current Value</div>
              <div className="text-2xl font-bold text-foreground">
                {alert.current_value.toFixed(2)}
                {alert.metric.includes('usage') && '%'}
              </div>
            </div>
            <div className="bg-background rounded-lg p-4 border border-border">
              <div className="text-sm text-muted-foreground mb-1">Threshold</div>
              <div className="text-2xl font-bold text-foreground">
                {alert.condition === 'greater_than' && '> '}
                {alert.condition === 'less_than' && '< '}
                {alert.condition === 'equals' && '= '}
                {alert.condition === 'not_equals' && '≠ '}
                {alert.threshold}
                {alert.metric.includes('usage') && '%'}
              </div>
            </div>
          </div>

          {/* Agent Info */}
          <div className="bg-background rounded-lg p-4 border border-border">
            <h3 className="text-lg font-semibold text-foreground mb-3 flex items-center gap-2">
              <Info className="w-5 h-5" />
              Agent Information
            </h3>
            <div className="space-y-2">
              <div className="flex justify-between">
                <span className="text-muted-foreground">Agent Name:</span>
                <span className="text-foreground font-medium">{alert.agent_name}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Agent ID:</span>
                <span className="text-foreground font-mono text-sm">{alert.agent_id}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">Metric:</span>
                <span className="text-foreground font-medium">{alert.metric}</span>
              </div>
            </div>
          </div>

          {/* Timeline */}
          <div className="bg-background rounded-lg p-4 border border-border">
            <h3 className="text-lg font-semibold text-foreground mb-3 flex items-center gap-2">
              <Clock className="w-5 h-5" />
              Timeline
            </h3>
            <div className="space-y-3">
              <div className="flex items-start gap-3">
                <div className="w-2 h-2 rounded-full bg-yellow-500 mt-2"></div>
                <div className="flex-1">
                  <div className="text-foreground font-medium">Alert Triggered</div>
                  <div className="text-sm text-muted-foreground">{formatTimestamp(alert.triggered_at)}</div>
                </div>
              </div>

              {alert.fired_at && (
                <div className="flex items-start gap-3">
                  <div className="w-2 h-2 rounded-full bg-red-500 mt-2"></div>
                  <div className="flex-1">
                    <div className="text-foreground font-medium">Alert Fired</div>
                    <div className="text-sm text-muted-foreground">{formatTimestamp(alert.fired_at)}</div>
                    <div className="text-xs text-muted-foreground">
                      Duration: {getDuration(alert.triggered_at, alert.fired_at)}
                    </div>
                  </div>
                </div>
              )}

              {alert.acknowledged && alert.acknowledged_at && (
                <div className="flex items-start gap-3">
                  <div className="w-2 h-2 rounded-full bg-blue-500 mt-2"></div>
                  <div className="flex-1">
                    <div className="text-foreground font-medium">Acknowledged</div>
                    <div className="text-sm text-muted-foreground">{formatTimestamp(alert.acknowledged_at)}</div>
                    {alert.acknowledged_by && (
                      <div className="text-sm text-muted-foreground">By: {alert.acknowledged_by}</div>
                    )}
                  </div>
                </div>
              )}

              {alert.resolved_at && (
                <div className="flex items-start gap-3">
                  <div className="w-2 h-2 rounded-full bg-green-500 mt-2"></div>
                  <div className="flex-1">
                    <div className="text-foreground font-medium">Resolved</div>
                    <div className="text-sm text-muted-foreground">{formatTimestamp(alert.resolved_at)}</div>
                    <div className="text-xs text-muted-foreground">
                      Total duration: {getDuration(alert.triggered_at, alert.resolved_at)}
                    </div>
                  </div>
                </div>
              )}

              {!alert.resolved_at && (
                <div className="flex items-start gap-3">
                  <div className="w-2 h-2 rounded-full bg-primary animate-pulse mt-2"></div>
                  <div className="flex-1">
                    <div className="text-foreground font-medium">Currently Active</div>
                    <div className="text-sm text-muted-foreground">
                      Duration: {getDuration(alert.triggered_at)}
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Resolution Info */}
          {alert.state.toLowerCase() !== 'resolved' && (
            <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4">
              <h3 className="text-lg font-semibold text-blue-500 mb-2 flex items-center gap-2">
                <Info className="w-5 h-5" />
                How to Resolve This Alert
              </h3>
              <div className="text-sm text-foreground space-y-2">
                <p>This alert will automatically resolve when:</p>
                <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                  <li>The metric value returns to normal (below/above threshold)</li>
                  <li>The condition is no longer met for at least one evaluation cycle (10 seconds)</li>
                </ul>
                <p className="mt-3 text-muted-foreground">
                  <strong>Current Status:</strong> {alert.metric} is {alert.current_value.toFixed(2)}
                  {alert.metric.includes('usage') && '%'}, threshold is {alert.condition} {alert.threshold}
                  {alert.metric.includes('usage') && '%'}
                </p>
                <p className="mt-2 text-yellow-500 text-xs">
                  💡 The system checks every 10 seconds. Once the condition clears, the alert will automatically resolve.
                </p>
              </div>
            </div>
          )}

          {/* Acknowledge Section */}
          {!alert.acknowledged && alert.state.toLowerCase() !== 'resolved' && (
            <div className="bg-background rounded-lg p-4 border border-border">
              {!showAcknowledgeForm ? (
                <button
                  onClick={() => setShowAcknowledgeForm(true)}
                  className="w-full py-3 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 transition-smooth flex items-center justify-center gap-2"
                >
                  <CheckCircle className="w-5 h-5" />
                  Acknowledge Alert
                </button>
              ) : (
                <div className="space-y-3">
                  <h3 className="text-lg font-semibold text-foreground">Acknowledge Alert</h3>
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-2">
                      Your Name *
                    </label>
                    <input
                      type="text"
                      value={acknowledgeBy}
                      onChange={(e) => setAcknowledgeBy(e.target.value)}
                      placeholder="e.g., John Doe"
                      className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                    />
                  </div>
                  <div>
                    <label className="block text-sm font-medium text-foreground mb-2">
                      Note (Optional)
                    </label>
                    <textarea
                      value={acknowledgeNote}
                      onChange={(e) => setAcknowledgeNote(e.target.value)}
                      placeholder="Add a note about this acknowledgment..."
                      rows={3}
                      className="w-full px-4 py-2 bg-background border border-border rounded-lg text-foreground focus:outline-none focus:ring-2 focus:ring-primary"
                    />
                  </div>
                  <div className="flex gap-3">
                    <button
                      onClick={handleAcknowledge}
                      className="flex-1 py-2 bg-primary text-primary-foreground rounded-lg font-medium hover:bg-primary/90 transition-smooth"
                    >
                      Confirm
                    </button>
                    <button
                      onClick={() => setShowAcknowledgeForm(false)}
                      className="px-6 py-2 bg-background border border-border rounded-lg font-medium text-foreground hover:bg-accent transition-smooth"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </div>
          )}

          {alert.acknowledged && (
            <div className="bg-green-500/10 border border-green-500/20 rounded-lg p-4 flex items-center gap-3">
              <CheckCircle className="w-5 h-5 text-green-500" />
              <div>
                <div className="text-foreground font-medium">Alert Acknowledged</div>
                <div className="text-sm text-muted-foreground">
                  By {alert.acknowledged_by} at {formatTimestamp(alert.acknowledged_at)}
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="sticky bottom-0 bg-card border-t border-border p-4 flex justify-end">
          <button
            onClick={onClose}
            className="px-6 py-2 bg-background border border-border rounded-lg font-medium text-foreground hover:bg-accent transition-smooth"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  )
}

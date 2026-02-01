'use client'

import { useState, useEffect } from 'react'
import Link from 'next/link'
import { Shield, Bell, AlertTriangle, CheckCircle, Clock, Settings, RefreshCw, XCircle, Activity, Trash2 } from 'lucide-react'
import { getAlerts, getAlertRules, Alert, AlertRule, deleteAlertRule, toggleAlertRule } from '@/lib/alerts-api'

export default function AdminAlertsPage() {
  const [alerts, setAlerts] = useState<Alert[]>([])
  const [rules, setRules] = useState<AlertRule[]>([])
  const [loading, setLoading] = useState(true)
  const [activeTab, setActiveTab] = useState<'alerts' | 'rules'>('alerts')

  useEffect(() => {
    loadData()
  }, [])

  const loadData = async () => {
    try {
      setLoading(true)
      const [alertsData, rulesData] = await Promise.all([
        getAlerts(),
        getAlertRules()
      ])
      
      const allAlerts = [...alertsData.active_alerts, ...alertsData.recent_alerts]
      setAlerts(allAlerts)
      setRules(rulesData.rules)
    } catch (err) {
      console.error('Failed to load alerts:', err)
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteRule = async (ruleId: string, ruleName: string) => {
    if (!confirm(`Are you sure you want to delete the alert rule "${ruleName}"?`)) {
      return
    }

    try {
      await deleteAlertRule(ruleId)
      await loadData()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to delete alert rule')
    }
  }

  const handleToggleRule = async (ruleId: string) => {
    try {
      await toggleAlertRule(ruleId)
      await loadData()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to toggle alert rule')
    }
  }

  const activeAlerts = alerts.filter((a: Alert) => a.state === 'firing')
  const pendingAlerts = alerts.filter((a: Alert) => a.state === 'pending')
  const resolvedAlerts = alerts.filter((a: Alert) => a.state === 'resolved')
  const enabledRules = rules.filter((r: AlertRule) => r.enabled)
  const disabledRules = rules.filter((r: AlertRule) => !r.enabled)

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleString()
  }

  const getSeverityColor = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'critical':
        return 'bg-red-500/20 text-red-500'
      case 'warning':
        return 'bg-yellow-500/20 text-yellow-500'
      case 'info':
        return 'bg-blue-500/20 text-blue-500'
      default:
        return 'bg-gray-500/20 text-gray-500'
    }
  }

  const getStateColor = (state: string) => {
    switch (state) {
      case 'firing':
        return 'bg-red-500/20 text-red-500'
      case 'pending':
        return 'bg-yellow-500/20 text-yellow-500'
      case 'resolved':
        return 'bg-green-500/20 text-green-500'
      default:
        return 'bg-gray-500/20 text-gray-500'
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-background p-8 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-foreground flex items-center gap-3">
              <Shield className="w-8 h-8 text-primary" />
              Alert Management
            </h1>
            <p className="text-muted-foreground mt-2">
              Monitor alerts and manage alert rules
            </p>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={loadData}
              className="px-4 py-2 bg-sidebar-accent/20 text-foreground rounded-lg hover:bg-sidebar-accent/30 transition-colors flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Refresh
            </button>
            <Link
              href="/alerts/rules/new"
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors flex items-center gap-2"
            >
              <Bell className="w-4 h-4" />
              New Alert Rule
            </Link>
          </div>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mb-8">
          <div className="p-4 rounded-lg bg-red-500/10 border border-red-500/30">
            <div className="flex items-center gap-2 mb-1">
              <AlertTriangle className="w-4 h-4 text-red-500" />
              <p className="text-sm text-red-400">Firing</p>
            </div>
            <p className="text-2xl font-bold text-red-500">{activeAlerts.length}</p>
          </div>
          
          <div className="p-4 rounded-lg bg-yellow-500/10 border border-yellow-500/30">
            <div className="flex items-center gap-2 mb-1">
              <Clock className="w-4 h-4 text-yellow-500" />
              <p className="text-sm text-yellow-400">Pending</p>
            </div>
            <p className="text-2xl font-bold text-yellow-500">{pendingAlerts.length}</p>
          </div>
          
          <div className="p-4 rounded-lg bg-green-500/10 border border-green-500/30">
            <div className="flex items-center gap-2 mb-1">
              <CheckCircle className="w-4 h-4 text-green-500" />
              <p className="text-sm text-green-400">Resolved</p>
            </div>
            <p className="text-2xl font-bold text-green-500">{resolvedAlerts.length}</p>
          </div>
          
          <div className="p-4 rounded-lg bg-blue-500/10 border border-blue-500/30">
            <div className="flex items-center gap-2 mb-1">
              <Activity className="w-4 h-4 text-blue-500" />
              <p className="text-sm text-blue-400">Total Alerts</p>
            </div>
            <p className="text-2xl font-bold text-blue-500">{alerts.length}</p>
          </div>
          
          <div className="p-4 rounded-lg bg-purple-500/10 border border-purple-500/30">
            <div className="flex items-center gap-2 mb-1">
              <Settings className="w-4 h-4 text-purple-500" />
              <p className="text-sm text-purple-400">Enabled Rules</p>
            </div>
            <p className="text-2xl font-bold text-purple-500">{enabledRules.length}</p>
          </div>
          
          <div className="p-4 rounded-lg bg-gray-500/10 border border-gray-500/30">
            <div className="flex items-center gap-2 mb-1">
              <XCircle className="w-4 h-4 text-gray-500" />
              <p className="text-sm text-gray-400">Disabled Rules</p>
            </div>
            <p className="text-2xl font-bold text-gray-500">{disabledRules.length}</p>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-4 mb-6 border-b border-card-border">
          <button
            onClick={() => setActiveTab('alerts')}
            className={`pb-4 px-2 font-medium transition-colors ${
              activeTab === 'alerts' 
                ? 'text-primary border-b-2 border-primary' 
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Active Alerts ({activeAlerts.length + pendingAlerts.length})
          </button>
          <button
            onClick={() => setActiveTab('rules')}
            className={`pb-4 px-2 font-medium transition-colors ${
              activeTab === 'rules' 
                ? 'text-primary border-b-2 border-primary' 
                : 'text-muted-foreground hover:text-foreground'
            }`}
          >
            Alert Rules ({rules.length})
          </button>
        </div>

        {/* Content */}
        {activeTab === 'alerts' ? (
          <div className="bg-card border border-card-border rounded-lg overflow-hidden">
            <table className="w-full">
              <thead>
                <tr className="border-b border-card-border bg-sidebar-accent/10">
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Alert</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Agent</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Severity</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">State</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Fired At</th>
                </tr>
              </thead>
              <tbody>
                {[...activeAlerts, ...pendingAlerts].length === 0 ? (
                  <tr>
                    <td colSpan={5} className="py-12 text-center text-muted-foreground">
                      <CheckCircle className="w-12 h-12 mx-auto mb-4 text-green-500/50" />
                      <p>No active alerts</p>
                    </td>
                  </tr>
                ) : (
                  [...activeAlerts, ...pendingAlerts].map((alert) => (
                    <tr key={alert.id} className="border-b border-card-border hover:bg-sidebar-accent/5">
                      <td className="py-4 px-6">
                        <p className="text-sm font-medium text-foreground">{alert.rule_name}</p>
                        <p className="text-xs text-muted-foreground">{alert.message}</p>
                      </td>
                      <td className="py-4 px-6 text-sm text-muted-foreground">
                        {alert.agent_id}
                      </td>
                      <td className="py-4 px-6">
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(alert.severity)}`}>
                          {alert.severity}
                        </span>
                      </td>
                      <td className="py-4 px-6">
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStateColor(alert.state)}`}>
                          {alert.state}
                        </span>
                      </td>
                      <td className="py-4 px-6 text-sm text-muted-foreground">
                        {formatDate(alert.fired_at)}
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="bg-card border border-card-border rounded-lg overflow-hidden">
            <table className="w-full">
              <thead>
                <tr className="border-b border-card-border bg-sidebar-accent/10">
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Rule Name</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Metric</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Condition</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Severity</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Status</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Actions</th>
                </tr>
              </thead>
              <tbody>
                {rules.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="py-12 text-center text-muted-foreground">
                      <Settings className="w-12 h-12 mx-auto mb-4 opacity-50" />
                      <p>No alert rules configured</p>
                    </td>
                  </tr>
                ) : (
                  rules.map((rule) => (
                    <tr key={rule.id} className="border-b border-card-border hover:bg-sidebar-accent/5">
                      <td className="py-4 px-6">
                        <p className="text-sm font-medium text-foreground">{rule.name}</p>
                      </td>
                      <td className="py-4 px-6 text-sm text-muted-foreground">
                        {rule.metric}
                      </td>
                      <td className="py-4 px-6 text-sm text-muted-foreground">
                        {rule.condition} {rule.threshold}
                      </td>
                      <td className="py-4 px-6">
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${getSeverityColor(rule.severity)}`}>
                          {rule.severity}
                        </span>
                      </td>
                      <td className="py-4 px-6">
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                          rule.enabled ? 'bg-green-500/20 text-green-500' : 'bg-gray-500/20 text-gray-500'
                        }`}>
                          {rule.enabled ? 'Enabled' : 'Disabled'}
                        </span>
                      </td>
                      <td className="py-4 px-6">
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => handleToggleRule(rule.id)}
                            className={`p-2 rounded-lg transition-colors ${
                              rule.enabled 
                                ? 'text-yellow-500 hover:bg-yellow-500/10' 
                                : 'text-green-500 hover:bg-green-500/10'
                            }`}
                            title={rule.enabled ? 'Disable' : 'Enable'}
                          >
                            {rule.enabled ? <XCircle className="w-4 h-4" /> : <CheckCircle className="w-4 h-4" />}
                          </button>
                          <button
                            onClick={() => handleDeleteRule(rule.id, rule.name)}
                            className="p-2 text-red-500 hover:bg-red-500/10 rounded-lg transition-colors"
                            title="Delete"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  )
}

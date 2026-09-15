'use client';

import { useEffect, useState, useCallback } from 'react';
import Link from 'next/link';
import { Bell, Plus, AlertTriangle, CheckCircle, Clock, Settings, Edit, Trash2, Power, Wifi, WifiOff } from 'lucide-react';
import { getAlerts, getAlertRules, acknowledgeAlert, deleteAlertRule, toggleAlertRule, Alert, AlertRule } from '@/lib/alerts-api';
import { AlertDetailModal } from '@/components/AlertDetailModal';
import { useAlertsWebSocket } from '@/hooks/useWebSocket';
import { WsAlertMessage } from '@/lib/websocket';
import AlertingOperations from '@/components/AlertingOperations';

export default function AlertsPage() {
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [rules, setRules] = useState<AlertRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<'active' | 'history' | 'rules'>('active');
  const [selectedAlert, setSelectedAlert] = useState<Alert | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);

  // Handle real-time alert updates via WebSocket
  const handleAlertMessage = useCallback((message: WsAlertMessage) => {
    // Refresh alerts when we receive a WebSocket alert
    fetchData();
  }, []);

  const { isConnected: wsConnected } = useAlertsWebSocket(handleAlertMessage);

  // Initial load and when filters change
  useEffect(() => {
    setIsAdmin(localStorage.getItem('user_type') === 'admin');
    fetchData();
  }, []);

  const fetchData = async () => {
    try {
      const isAdmin = localStorage.getItem('user_type') === 'admin';
      const alertsData = await getAlerts();
      const rulesData = isAdmin ? await getAlertRules() : { rules: [], total: 0 };
      
      const allAlerts = [...alertsData.active_alerts, ...alertsData.recent_alerts];
      setAlerts(allAlerts);
      setRules(rulesData.rules);
      setLoading(false);
    } catch (error) {
      console.error('Failed to fetch alerts:', error);
      setLoading(false);
    }
  };

  const handleAcknowledge = async (alertId: string) => {
    try {
      await acknowledgeAlert(alertId, 'User');
      fetchData();
    } catch (error) {
      console.error('Failed to acknowledge alert:', error);
    }
  };

  const activeAlerts = alerts.filter(a => a.state === 'firing');
  const pendingAlerts = alerts.filter(a => a.state === 'pending');
  const resolvedAlerts = alerts.filter(a => a.state === 'resolved');

  if (loading) {
    return (
      <div className="p-8">
        <div className="animate-pulse space-y-6">
          <div className="h-8 bg-gray-700 rounded w-1/4"></div>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            {[1, 2, 3].map(i => (
              <div key={i} className="h-32 bg-gray-800 rounded-xl"></div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <Bell className="w-8 h-8 text-primary" />
            <h1 className="text-3xl font-bold">Alerts</h1>
          </div>
          <p className="text-muted-foreground">Monitor and manage system alerts</p>
        </div>
        {isAdmin && (
          <Link
            href="/alerts/rules/new"
            className="flex items-center gap-2 px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-colors"
          >
            <Plus className="w-4 h-4" />
            New Alert Rule
          </Link>
        )}
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-6">
        {isAdmin && <div className="glass-morphism rounded-xl p-4">
          <div className="flex items-center gap-2 mb-2">
            <AlertTriangle className="w-5 h-5 text-red-400" />
            <span className="text-sm text-muted-foreground">Firing</span>
          </div>
          <div className="text-3xl font-bold text-red-400">{activeAlerts.length}</div>
        </div>}
        
        <div className="glass-morphism rounded-xl p-4">
          <div className="flex items-center gap-2 mb-2">
            <Clock className="w-5 h-5 text-yellow-400" />
            <span className="text-sm text-muted-foreground">Pending</span>
          </div>
          <div className="text-3xl font-bold text-yellow-400">{pendingAlerts.length}</div>
        </div>
        
        <div className="glass-morphism rounded-xl p-4">
          <div className="flex items-center gap-2 mb-2">
            <CheckCircle className="w-5 h-5 text-green-400" />
            <span className="text-sm text-muted-foreground">Resolved</span>
          </div>
          <div className="text-3xl font-bold text-green-400">{resolvedAlerts.length}</div>
        </div>
        
        <div className="glass-morphism rounded-xl p-4">
          <div className="flex items-center gap-2 mb-2">
            <Settings className="w-5 h-5 text-blue-400" />
            <span className="text-sm text-muted-foreground">Rules</span>
          </div>
          <div className="text-3xl font-bold text-blue-400">{rules.length}</div>
        </div>
      </div>

      {isAdmin && <AlertingOperations rules={rules} />}

      {/* Tabs */}
      <div className="flex gap-2 mb-6 border-b border-gray-700">
        {isAdmin && <button
          onClick={() => setActiveTab('active')}
          className={`px-4 py-2 font-medium transition-colors ${
            activeTab === 'active'
              ? 'text-primary border-b-2 border-primary'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          Active ({activeAlerts.length + pendingAlerts.length})
        </button>}
        <button
          onClick={() => setActiveTab('history')}
          className={`px-4 py-2 font-medium transition-colors ${
            activeTab === 'history'
              ? 'text-primary border-b-2 border-primary'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          History ({resolvedAlerts.length})
        </button>
        <button
          onClick={() => setActiveTab('rules')}
          className={`px-4 py-2 font-medium transition-colors ${
            activeTab === 'rules'
              ? 'text-primary border-b-2 border-primary'
              : 'text-muted-foreground hover:text-foreground'
          }`}
        >
          Rules ({rules.length})
        </button>
      </div>

      {/* Content */}
      {activeTab === 'active' && (
        <div className="space-y-4">
          {activeAlerts.length === 0 && pendingAlerts.length === 0 ? (
            <div className="glass-morphism rounded-xl p-12 text-center">
              <CheckCircle className="w-16 h-16 text-green-400 mx-auto mb-4" />
              <h3 className="text-xl font-semibold mb-2">All Clear!</h3>
              <p className="text-muted-foreground">No active alerts at the moment</p>
            </div>
          ) : (
            <>
              {activeAlerts.map(alert => (
                <AlertCard 
                  key={alert.id} 
                  alert={alert} 
                  onAcknowledge={handleAcknowledge}
                  onClick={() => setSelectedAlert(alert)}
                />
              ))}
              {pendingAlerts.map(alert => (
                <AlertCard 
                  key={alert.id} 
                  alert={alert} 
                  onAcknowledge={handleAcknowledge}
                  onClick={() => setSelectedAlert(alert)}
                />
              ))}
            </>
          )}
        </div>
      )}

      {activeTab === 'history' && (
        <div className="space-y-4">
          {resolvedAlerts.length === 0 ? (
            <div className="glass-morphism rounded-xl p-12 text-center">
              <Clock className="w-16 h-16 text-gray-400 mx-auto mb-4" />
              <h3 className="text-xl font-semibold mb-2">No History</h3>
              <p className="text-muted-foreground">No resolved alerts yet</p>
            </div>
          ) : (
            resolvedAlerts.map(alert => (
              <AlertCard 
                key={alert.id} 
                alert={alert} 
                onAcknowledge={handleAcknowledge}
                onClick={() => setSelectedAlert(alert)}
              />
            ))
          )}
        </div>
      )}

      {activeTab === 'rules' && (
        <div className="space-y-4">
          {rules.length === 0 ? (
            <div className="glass-morphism rounded-xl p-12 text-center">
              <Settings className="w-16 h-16 text-gray-400 mx-auto mb-4" />
              <h3 className="text-xl font-semibold mb-2">No Alert Rules</h3>
              <p className="text-muted-foreground mb-4">Create your first alert rule to start monitoring</p>
              <Link
                href="/alerts/rules/new"
                className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-white rounded-lg hover:bg-primary/90 transition-colors"
              >
                <Plus className="w-4 h-4" />
                Create Alert Rule
              </Link>
            </div>
          ) : (
            rules.map(rule => (
              <RuleCard key={rule.id} rule={rule} onUpdate={fetchData} />
            ))
          )}
        </div>
      )}
      
      {/* Alert Detail Modal */}
      {selectedAlert && (
        <AlertDetailModal
          alert={selectedAlert}
          onClose={() => setSelectedAlert(null)}
          onAcknowledge={() => {
            setSelectedAlert(null)
            fetchData()
          }}
        />
      )}
    </div>
  );
}

function AlertCard({ alert, onAcknowledge, onClick }: { 
  alert: Alert; 
  onAcknowledge: (id: string) => void;
  onClick?: () => void;
}) {
  const severityColors = {
    info: 'border-blue-500/30 bg-blue-500/5',
    warning: 'border-yellow-500/30 bg-yellow-500/5',
    critical: 'border-red-500/30 bg-red-500/5',
  };

  const stateColors = {
    pending: 'text-yellow-400',
    firing: 'text-red-400',
    resolved: 'text-green-400',
  };

  const severityIcons = {
    info: 'ℹ️',
    warning: '⚠️',
    critical: '🚨',
  };

  return (
    <div 
      className={`glass-morphism rounded-xl p-6 border ${severityColors[alert.severity]} cursor-pointer hover:border-primary/50 transition-all`}
      onClick={onClick}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <span className="text-2xl">{severityIcons[alert.severity]}</span>
          <div>
            <h3 className="text-lg font-semibold">{alert.rule_name}</h3>
            <p className="text-sm text-muted-foreground">
              {alert.agent_name} • {new Date(alert.triggered_at).toLocaleString()}
            </p>
          </div>
        </div>
        <span className={`px-3 py-1 rounded-full text-sm font-medium ${stateColors[alert.state]}`}>
          {alert.state.toUpperCase()}
        </span>
      </div>

      <p className="text-foreground mb-4">{alert.message}</p>

      <div className="flex items-center justify-between text-sm">
        <div className="flex gap-4">
          <span className="text-muted-foreground">
            <strong>Metric:</strong> {alert.metric}
          </span>
          <span className="text-muted-foreground">
            <strong>Value:</strong> {alert.current_value.toFixed(2)}
          </span>
          <span className="text-muted-foreground">
            <strong>Threshold:</strong> {alert.threshold}
          </span>
        </div>
        
        {alert.state === 'firing' && !alert.acknowledged && (
          <button
            onClick={(e) => {
              e.stopPropagation()
              onAcknowledge(alert.id)
            }}
            className="px-4 py-2 bg-primary/20 text-primary rounded-lg hover:bg-primary/30 transition-colors"
          >
            Acknowledge
          </button>
        )}
        
        {alert.acknowledged && (
          <span className="text-sm text-green-400">
            ✓ Acknowledged by {alert.acknowledged_by}
          </span>
        )}
      </div>
    </div>
  );
}

function RuleCard({ rule, onUpdate }: { rule: AlertRule; onUpdate: () => void }) {
  const [deleting, setDeleting] = useState(false)
  const [toggling, setToggling] = useState(false)
  
  const severityColors = {
    info: 'text-blue-400',
    warning: 'text-yellow-400',
    critical: 'text-red-400',
  };

  const handleDelete = async () => {
    if (!window.confirm('Are you sure you want to delete this alert rule?')) return
    
    setDeleting(true)
    try {
      await deleteAlertRule(rule.id)
      onUpdate()
    } catch (error) {
      console.error('Failed to delete rule:', error)
      window.alert('Failed to delete rule')
    } finally {
      setDeleting(false)
    }
  }

  const handleToggle = async () => {
    setToggling(true)
    try {
      await toggleAlertRule(rule.id)
      onUpdate()
    } catch (error) {
      console.error('Failed to toggle rule:', error)
      window.alert('Failed to toggle rule')
    } finally {
      setToggling(false)
    }
  }

  return (
    <div className="glass-morphism rounded-xl p-6">
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          <h3 className="text-lg font-semibold">{rule.name}</h3>
          {rule.description && (
            <p className="text-sm text-muted-foreground mt-1">{rule.description}</p>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${severityColors[rule.severity]}`}>
            {rule.severity.toUpperCase()}
          </span>
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
            rule.enabled ? 'bg-green-500/20 text-green-400' : 'bg-gray-500/20 text-gray-400'
          }`}>
            {rule.enabled ? 'ENABLED' : 'DISABLED'}
          </span>
        </div>
      </div>

      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm mb-4">
        <div>
          <span className="text-muted-foreground">Metric</span>
          <p className="font-medium">{rule.metric}</p>
        </div>
        <div>
          <span className="text-muted-foreground">Condition</span>
          <p className="font-medium">{rule.condition} {rule.threshold}</p>
        </div>
        <div>
          <span className="text-muted-foreground">Duration</span>
          <p className="font-medium">{rule.duration_seconds}s</p>
        </div>
        <div>
          <span className="text-muted-foreground">Cooldown</span>
          <p className="font-medium">{rule.cooldown_seconds}s</p>
        </div>
      </div>
      <p className="mb-4 text-xs text-muted-foreground">
        {rule.channels.length > 0 ? `${rule.channels.length} notification destination${rule.channels.length === 1 ? '' : 's'}` : 'Dashboard only — no external notification destination'}
      </p>
      
      <div className="flex gap-2 pt-4 border-t border-border">
        <button
          onClick={handleToggle}
          disabled={toggling}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg transition-colors disabled:opacity-50 ${
            rule.enabled
              ? 'bg-gray-500/20 text-gray-400 hover:bg-gray-500/30'
              : 'bg-green-500/20 text-green-400 hover:bg-green-500/30'
          }`}
        >
          <Power className="w-4 h-4" />
          {toggling ? 'Updating...' : rule.enabled ? 'Disable' : 'Enable'}
        </button>
        <Link
          href={`/alerts/rules/edit/${rule.id}`}
          className="flex items-center gap-2 px-4 py-2 bg-primary/20 text-primary rounded-lg hover:bg-primary/30 transition-colors"
        >
          <Edit className="w-4 h-4" />
          Edit
        </Link>
        <button
          onClick={handleDelete}
          disabled={deleting}
          className="flex items-center gap-2 px-4 py-2 bg-red-500/20 text-red-400 rounded-lg hover:bg-red-500/30 transition-colors disabled:opacity-50"
        >
          <Trash2 className="w-4 h-4" />
          {deleting ? 'Deleting...' : 'Delete'}
        </button>
      </div>
    </div>
  );
}

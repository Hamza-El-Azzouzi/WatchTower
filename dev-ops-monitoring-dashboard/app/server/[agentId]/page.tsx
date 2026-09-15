'use client';

import { useEffect, useState, useCallback } from 'react';
import { useParams } from 'next/navigation';
import Link from 'next/link';
import { ArrowLeft, AlertCircle, Loader, Radio, ShieldCheck } from 'lucide-react';
import StatusBadge from '@/components/StatusBadge';
import MetricsSection from '@/components/MetricsSection';
import AlertThresholdChart from '@/components/AlertThresholdChart';
import ConnectionStatus from '@/components/ConnectionStatus';
import ProcessWatch from '@/components/ProcessWatch';
import HostTelemetryPanel from '@/components/HostTelemetryPanel';
import IncidentTimeline from '@/components/IncidentTimeline';
import { getEffectiveAlertRules, type AlertRule } from '@/lib/alerts-api';
import { formatRelativeTime } from '@/lib/metrics-utils';
import { Agent, LatestMetrics, Metric } from '@/types';
import { useMetricsWebSocket } from '@/hooks/useWebSocket';
import { HostTelemetrySnapshot, ProcessSnapshot, WsHostTelemetryMessage, WsMetricMessage, WsAgentSnapshot, WsMetricSnapshot, WsProcessSnapshotMessage } from '@/lib/websocket';

export default function ServerDetailPage() {
  const params = useParams();
  const agentId = decodeURIComponent(params.agentId as string);

  const [agent, setAgent] = useState<Agent | null>(null);
  const [metrics, setMetrics] = useState<LatestMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState(new Date());
  const [processes, setProcesses] = useState<ProcessSnapshot[]>([]);
  const [processesUpdatedAt, setProcessesUpdatedAt] = useState<string | null>(null);
  const [hostTelemetry, setHostTelemetry] = useState<HostTelemetrySnapshot | null>(null);
  const [alertRules, setAlertRules] = useState<AlertRule[]>([]);

  useEffect(() => {
    let active = true;
    getEffectiveAlertRules(agentId)
      .then(result => { if (active) setAlertRules(result.rules); })
      .catch(error => console.error('Failed to load effective alert rules:', error));
    return () => { active = false; };
  }, [agentId]);

  const thresholdsFor = useCallback((metric: string) => {
    const metricRules = alertRules.filter(rule => rule.metric === metric && rule.condition === 'greater_than');
    return {
      warning: metricRules.filter(rule => rule.severity === 'warning').map(rule => rule.threshold).sort((a, b) => a - b)[0],
      critical: metricRules.filter(rule => rule.severity === 'critical').map(rule => rule.threshold).sort((a, b) => a - b)[0],
    };
  }, [alertRules]);

  // Handle initial state from WebSocket - replaces HTTP fetch
  const handleInitialState = useCallback((agents: WsAgentSnapshot[], metricsSnapshots: WsMetricSnapshot[], telemetry: HostTelemetrySnapshot[]) => {
    console.log('[ServerDetail] Received initial state');
    
    // Find this agent
    const foundAgent = agents.find(a => a.id === agentId);
    if (foundAgent) {
      setAgent({
        id: foundAgent.id,
        name: foundAgent.name,
        status: foundAgent.status as 'Healthy' | 'Degraded' | 'Unreachable',
        last_seen: foundAgent.last_seen,
      });
    }
    
    // Get metrics for this agent
    const agentMetrics = metricsSnapshots.filter(m => m.agent_id === agentId);
    if (agentMetrics.length > 0) {
      setMetrics({
        agent_id: agentId,
        metrics: agentMetrics.map(m => ({
          name: m.metric_name,
          value: m.latest_value,
          timestamp: m.timestamp,
        })),
      });
    }
    setHostTelemetry(telemetry.find(snapshot => snapshot.agent_id === agentId) ?? null);
    
    setLoading(false);
    setLastUpdated(new Date());
  }, [agentId]);

  // Handle real-time metric updates
  const handleMetricUpdate = useCallback((metricMessage: WsMetricMessage) => {
    if (metricMessage.agent_id !== agentId) return;
    
    setLastUpdated(new Date());
    
    // Update the metrics state with new metric values as they arrive
    setMetrics(prev => {
      if (!prev) {
        return {
          agent_id: agentId,
          metrics: [{
            name: metricMessage.metric_name,
            value: metricMessage.value,
            timestamp: metricMessage.timestamp,
          }],
        };
      }
      
      const newMetric: Metric = {
        name: metricMessage.metric_name,
        value: metricMessage.value,
        timestamp: metricMessage.timestamp,
      };
      if (!newMetric.name) return prev;
      
      // Find and replace the metric or add it
      const existingIndex = prev.metrics.findIndex(m => m.name === metricMessage.metric_name);
      
      const updatedMetrics = [...prev.metrics];
      if (existingIndex >= 0) {
        updatedMetrics[existingIndex] = newMetric;
      } else {
        updatedMetrics.push(newMetric);
      }
      
      return {
        ...prev,
        metrics: updatedMetrics,
      };
    });
    
    // Also update agent's last_seen
    setAgent(prev => prev ? { ...prev, last_seen: metricMessage.timestamp } : prev);
  }, [agentId]);

  const handleProcessSnapshot = useCallback((message: WsProcessSnapshotMessage) => {
    if (message.agent_id !== agentId) return;
    setProcesses(message.processes);
    setProcessesUpdatedAt(message.timestamp);
  }, [agentId]);

  const handleHostTelemetry = useCallback((message: WsHostTelemetryMessage) => {
    if (message.agent_id !== agentId) return;
    setHostTelemetry(message);
  }, [agentId]);

  // Connect to WebSocket - this is the ONLY data source
  const { isConnected, connectionState, initialStateReceived } = useMetricsWebSocket({
    onMetric: handleMetricUpdate,
    onProcessSnapshot: handleProcessSnapshot,
    onHostTelemetry: handleHostTelemetry,
    onInitialState: handleInitialState,
  });

  // Check if agent not found after initial state
  useEffect(() => {
    if (initialStateReceived && !agent) {
      setError('Server not found');
      setLoading(false);
    }
  }, [initialStateReceived, agent]);

  return (
    <div className="min-h-screen">
      {/* Header */}
      <div className="sticky top-16 z-10 border-b border-white/8 bg-[#070b12]/80 backdrop-blur-2xl lg:top-0">
        <div className="px-4 py-5 sm:px-6 lg:px-10">
          <Link
            href="/"
            className="inline-flex items-center gap-2 text-accent hover:text-primary transition-smooth mb-4"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Overview
          </Link>

          {agent && (
            <div className="flex items-center justify-between">
              <div>
                <h1 className="text-3xl font-bold text-foreground">{agent.name}</h1>
                <p className="text-sm text-muted-foreground mt-1">
                  Last seen: {formatRelativeTime(agent.last_seen)}
                </p>
              </div>
              <StatusBadge status={agent.status} size="lg" />
            </div>
          )}
        </div>
      </div>

      {/* Content */}
      <main className="page-shell">
        {error && (
          <div className="mb-8 glass-morphism rounded-xl border border-red-500/30 bg-red-500/10 p-4 flex items-start gap-3 animate-slide-up">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <h3 className="font-semibold text-red-300 mb-1">Error</h3>
              <p className="text-sm text-red-200">{error}</p>
            </div>
          </div>
        )}

        {loading && !agent ? (
          <div className="flex flex-col items-center justify-center py-12">
            <Loader className="w-8 h-8 text-accent animate-spin mb-4" />
            <p className="text-muted-foreground">Loading server details...</p>
          </div>
        ) : agent ? (
          <>
            <h2 className="text-2xl font-bold text-foreground mb-6">Current Metrics</h2>
            <MetricsSection metrics={metrics} loading={loading} />

            <HostTelemetryPanel telemetry={hostTelemetry} metrics={metrics} />

            <h2 className="text-2xl font-bold text-foreground mb-6 mt-12">Process Explorer</h2>
            <ProcessWatch processes={processes} updatedAt={processesUpdatedAt} />

            <div className="mt-12">
              <IncidentTimeline agentId={agentId} />
            </div>

            <div className="mb-6 mt-12 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
              <div>
                <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-accent">
                  <ShieldCheck className="h-4 w-4" /> Operational guardrails
                </div>
                <h2 className="text-2xl font-bold text-foreground">Capacity & Alert Thresholds</h2>
                <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
                  Live saturation, headroom, and threshold proximity for the resources that most often cause incidents.
                </p>
              </div>
              <div className="inline-flex w-fit items-center gap-2 rounded-full border border-emerald-400/20 bg-emerald-400/10 px-3 py-1.5 text-xs font-medium text-emerald-300">
                <Radio className="h-3.5 w-3.5" /> 2 second live samples
              </div>
            </div>
            <div className="grid grid-cols-1 gap-8 mb-12">
              <AlertThresholdChart
                agentId={agentId}
                metric="cpu_usage"
                title="CPU utilization"
                warningThreshold={thresholdsFor('cpu_usage').warning}
                criticalThreshold={thresholdsFor('cpu_usage').critical}
                limit={120}
                liveMetric={metrics?.metrics.find(item => item.name === 'cpu_usage')}
              />
              <AlertThresholdChart
                agentId={agentId}
                metric="memory_usage"
                title="Memory pressure"
                warningThreshold={thresholdsFor('memory_usage').warning}
                criticalThreshold={thresholdsFor('memory_usage').critical}
                limit={120}
                liveMetric={metrics?.metrics.find(item => item.name === 'memory_usage')}
              />
              <AlertThresholdChart
                agentId={agentId}
                metric="disk_usage"
                title="Disk capacity"
                warningThreshold={thresholdsFor('disk_usage').warning}
                criticalThreshold={thresholdsFor('disk_usage').critical}
                limit={120}
                liveMetric={metrics?.metrics.find(item => item.name === 'disk_usage')}
              />
            </div>

            {/* WebSocket Status Footer */}
            <div className="mt-12 pt-8 border-t border-border flex items-center justify-between">
              <div className="text-xs text-muted-foreground flex items-center gap-2">
                <ConnectionStatus state={connectionState} />
                <span className="mx-2">•</span>
                Last updated: {lastUpdated.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
              </div>
            </div>
          </>
        ) : null}
      </main>
    </div>
  );
}

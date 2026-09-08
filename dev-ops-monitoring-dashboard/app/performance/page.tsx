'use client';

import { useState, useEffect, useCallback } from 'react';
import PageHeader from '@/components/PageHeader';
import { Zap, TrendingUp, TrendingDown, Activity, Server, AlertTriangle, Wifi } from 'lucide-react';
import Link from 'next/link';
import { useMetricsWebSocket } from '@/hooks/useWebSocket';
import { WsMetricMessage } from '@/lib/websocket';
import { getAgents, getLatestMetrics } from '@/lib/api';

interface Agent {
  id: string;
  name: string;
  last_seen: string;
}

interface AgentPerformance {
  agent_id: string;
  hostname: string;
  cpu: number;
  memory: number;
  disk: number;
  score: number;
  status: 'excellent' | 'good' | 'warning' | 'critical';
}

// Calculate performance score (0-100)
// CPU: 40%, Memory: 40%, Disk: 20%
function calculatePerformanceScore(cpu: number, memory: number, disk: number): number {
  const cpuScore = (100 - cpu) * 0.4;
  const memoryScore = (100 - memory) * 0.4;
  const diskScore = (100 - disk) * 0.2;
  return Math.round(cpuScore + memoryScore + diskScore);
}

function getStatusFromScore(score: number): 'excellent' | 'good' | 'warning' | 'critical' {
  if (score >= 80) return 'excellent';
  if (score >= 60) return 'good';
  if (score >= 40) return 'warning';
  return 'critical';
}

function getStatusColor(status: string): string {
  switch (status) {
    case 'excellent': return 'text-emerald-400 bg-emerald-400/20 border-emerald-400/30';
    case 'good': return 'text-blue-400 bg-blue-400/20 border-blue-400/30';
    case 'warning': return 'text-amber-400 bg-amber-400/20 border-amber-400/30';
    case 'critical': return 'text-red-400 bg-red-400/20 border-red-400/30';
    default: return 'text-gray-400 bg-gray-400/20 border-gray-400/30';
  }
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'excellent': return <TrendingUp className="w-4 h-4" />;
    case 'good': return <Activity className="w-4 h-4" />;
    case 'warning': return <TrendingDown className="w-4 h-4" />;
    case 'critical': return <AlertTriangle className="w-4 h-4" />;
    default: return <Activity className="w-4 h-4" />;
  }
}

export default function PerformancePage() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [performances, setPerformances] = useState<AgentPerformance[]>([]);
  const [loading, setLoading] = useState(true);

  // WebSocket handler for real-time performance updates
  const handleMetricUpdate = useCallback((metricMessage: WsMetricMessage) => {
    const agentId = metricMessage.agent_id;
    const metricName = metricMessage.metric_name;
    const value = metricMessage.value;

    setPerformances(prev => prev.map(perf => {
      if (perf.agent_id !== agentId) return perf;

      const updated = { ...perf };
      if (metricName === 'cpu_usage') updated.cpu = value;
      else if (metricName === 'memory_usage') updated.memory = value;
      else if (metricName === 'disk_usage') updated.disk = value;
      else return perf;

      updated.score = calculatePerformanceScore(updated.cpu, updated.memory, updated.disk);
      updated.status = getStatusFromScore(updated.score);
      return updated;
    }));
  }, []);

  const { isConnected: wsConnected } = useMetricsWebSocket(handleMetricUpdate);

  const fetchData = async () => {
    try {
      // Fetch agents
      const agentsData: Agent[] = await getAgents();
      setAgents(agentsData);

      // Fetch metrics for each agent
      const performanceList: AgentPerformance[] = [];

      for (const agent of agentsData) {
        try {
          const latest = await getLatestMetrics(agent.id);
          const valueOf = (name: string) => latest.metrics.find(metric => metric.name === name)?.value || 0;
          const cpu = valueOf('cpu_usage');
          const memory = valueOf('memory_usage');
          const disk = valueOf('disk_usage');

          const score = calculatePerformanceScore(cpu, memory, disk);
          const status = getStatusFromScore(score);

          performanceList.push({
            agent_id: agent.id,
            hostname: agent.name,
            cpu,
            memory,
            disk,
            score,
            status,
          });
        } catch (err) {
          console.error(`Failed to fetch metrics for ${agent.id}:`, err);
        }
      }

      // Sort by score (worst first for attention)
      performanceList.sort((a, b) => a.score - b.score);

      setPerformances(performanceList);
      setLoading(false);
    } catch (error) {
      console.error('Failed to fetch performance data:', error);
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchData();
  }, []);

  // No polling - WebSocket updates metrics in real-time across the app

  // Calculate fleet-wide statistics
  const fleetStats = {
    avgScore: performances.length > 0
      ? Math.round(performances.reduce((sum, p) => sum + p.score, 0) / performances.length)
      : 0,
    avgCpu: performances.length > 0
      ? Math.round(performances.reduce((sum, p) => sum + p.cpu, 0) / performances.length)
      : 0,
    avgMemory: performances.length > 0
      ? Math.round(performances.reduce((sum, p) => sum + p.memory, 0) / performances.length)
      : 0,
    avgDisk: performances.length > 0
      ? Math.round(performances.reduce((sum, p) => sum + p.disk, 0) / performances.length)
      : 0,
    excellent: performances.filter(p => p.status === 'excellent').length,
    good: performances.filter(p => p.status === 'good').length,
    warning: performances.filter(p => p.status === 'warning').length,
    critical: performances.filter(p => p.status === 'critical').length,
  };

  const fleetStatus = getStatusFromScore(fleetStats.avgScore);

  return (
    <div className="min-h-screen bg-background">
      <PageHeader />

      <main className="max-w-7xl mx-auto px-8 py-8">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div className="flex items-center gap-3">
            <div className="p-3 rounded-lg bg-accent/20 border border-accent/30">
              <Zap className="w-6 h-6 text-accent" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-foreground">Performance Analytics</h1>
              <p className="text-sm text-muted-foreground">Real-time fleet performance monitoring</p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            {wsConnected ? (
              <span className="flex items-center gap-2 px-3 py-1.5 bg-green-600/20 border border-green-600/30 rounded-lg text-green-400 text-sm">
                <Wifi className="w-4 h-4" />
                <span className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
                Live
              </span>
            ) : (
              <span className="flex items-center gap-2 px-3 py-1.5 bg-yellow-600/20 border border-yellow-600/30 rounded-lg text-yellow-400 text-sm">
                <Wifi className="w-4 h-4" />
                Connecting...
              </span>
            )}
            <button
              onClick={fetchData}
              disabled={loading}
              className="px-4 py-2 rounded-lg bg-accent/20 border border-accent/30 text-accent hover:bg-accent/30 transition-colors disabled:opacity-50"
            >
              {loading ? 'Loading...' : 'Refresh'}
            </button>
          </div>
        </div>

        {loading && performances.length === 0 ? (
          <div className="glass-morphism rounded-xl border border-border p-12 text-center">
            <div className="inline-block w-8 h-8 border-4 border-accent/30 border-t-accent rounded-full animate-spin mb-4"></div>
            <p className="text-muted-foreground">Loading performance data...</p>
          </div>
        ) : (
          <>
            {/* Fleet Summary Cards */}
            <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
              <div className="glass-morphism rounded-xl border border-border p-6">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Fleet Score</span>
                  <div className={`px-2 py-1 rounded-lg border text-xs font-medium ${getStatusColor(fleetStatus)}`}>
                    {fleetStatus.toUpperCase()}
                  </div>
                </div>
                <div className="flex items-end gap-2">
                  <div className="text-3xl font-bold text-foreground">{fleetStats.avgScore}</div>
                  <div className="text-sm text-muted-foreground mb-1">/100</div>
                </div>
                <p className="text-xs text-muted-foreground mt-2">Average performance score</p>
              </div>

              <div className="glass-morphism rounded-xl border border-border p-6">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Avg CPU</span>
                </div>
                <div className="flex items-end gap-2">
                  <div className="text-3xl font-bold text-foreground">{fleetStats.avgCpu}%</div>
                </div>
                <div className="mt-2 h-2 bg-border rounded-full overflow-hidden">
                  <div
                    className="h-full bg-accent transition-all"
                    style={{ width: `${fleetStats.avgCpu}%` }}
                  />
                </div>
              </div>

              <div className="glass-morphism rounded-xl border border-border p-6">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Avg Memory</span>
                </div>
                <div className="flex items-end gap-2">
                  <div className="text-3xl font-bold text-foreground">{fleetStats.avgMemory}%</div>
                </div>
                <div className="mt-2 h-2 bg-border rounded-full overflow-hidden">
                  <div
                    className="h-full bg-primary transition-all"
                    style={{ width: `${fleetStats.avgMemory}%` }}
                  />
                </div>
              </div>

              <div className="glass-morphism rounded-xl border border-border p-6">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-muted-foreground">Avg Disk</span>
                </div>
                <div className="flex items-end gap-2">
                  <div className="text-3xl font-bold text-foreground">{fleetStats.avgDisk}%</div>
                </div>
                <div className="mt-2 h-2 bg-border rounded-full overflow-hidden">
                  <div
                    className="h-full bg-emerald-400 transition-all"
                    style={{ width: `${fleetStats.avgDisk}%` }}
                  />
                </div>
              </div>
            </div>

            {/* Health Distribution */}
            <div className="glass-morphism rounded-xl border border-border p-6 mb-8">
              <h2 className="text-lg font-semibold text-foreground mb-4">Fleet Health Distribution</h2>
              <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm text-emerald-400 font-medium">Excellent</span>
                      <span className="text-sm text-muted-foreground">{fleetStats.excellent}</span>
                    </div>
                    <div className="h-2 bg-border rounded-full overflow-hidden">
                      <div
                        className="h-full bg-emerald-400 transition-all"
                        style={{ width: `${performances.length > 0 ? (fleetStats.excellent / performances.length) * 100 : 0}%` }}
                      />
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm text-blue-400 font-medium">Good</span>
                      <span className="text-sm text-muted-foreground">{fleetStats.good}</span>
                    </div>
                    <div className="h-2 bg-border rounded-full overflow-hidden">
                      <div
                        className="h-full bg-blue-400 transition-all"
                        style={{ width: `${performances.length > 0 ? (fleetStats.good / performances.length) * 100 : 0}%` }}
                      />
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm text-amber-400 font-medium">Warning</span>
                      <span className="text-sm text-muted-foreground">{fleetStats.warning}</span>
                    </div>
                    <div className="h-2 bg-border rounded-full overflow-hidden">
                      <div
                        className="h-full bg-amber-400 transition-all"
                        style={{ width: `${performances.length > 0 ? (fleetStats.warning / performances.length) * 100 : 0}%` }}
                      />
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="flex items-center justify-between mb-1">
                      <span className="text-sm text-red-400 font-medium">Critical</span>
                      <span className="text-sm text-muted-foreground">{fleetStats.critical}</span>
                    </div>
                    <div className="h-2 bg-border rounded-full overflow-hidden">
                      <div
                        className="h-full bg-red-400 transition-all"
                        style={{ width: `${performances.length > 0 ? (fleetStats.critical / performances.length) * 100 : 0}%` }}
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Agent Performance Rankings */}
            <div className="glass-morphism rounded-xl border border-border p-6">
              <h2 className="text-lg font-semibold text-foreground mb-4">Agent Performance Rankings</h2>
              {performances.length === 0 ? (
                <p className="text-center text-muted-foreground py-8">No agents available</p>
              ) : (
                <div className="space-y-4">
                  {performances.map((perf, index) => (
                    <Link
                      key={perf.agent_id}
                      href={`/server/${perf.agent_id}`}
                      className="block group"
                    >
                      <div className="p-4 rounded-lg border border-border hover:border-accent/50 transition-colors bg-background/50">
                        <div className="flex items-center gap-4">
                          {/* Rank */}
                          <div className="flex-shrink-0 w-8 text-center">
                            <div className={`text-lg font-bold ${
                              index === 0 ? 'text-red-400' :
                              index === 1 ? 'text-amber-400' :
                              index === 2 ? 'text-yellow-400' :
                              'text-muted-foreground'
                            }`}>
                              #{index + 1}
                            </div>
                          </div>

                          {/* Server Info */}
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-3 mb-2">
                              <Server className="w-4 h-4 text-muted-foreground flex-shrink-0" />
                              <div className="font-medium text-foreground truncate">{perf.hostname}</div>
                              <div className={`px-2 py-1 rounded-lg border text-xs font-medium flex items-center gap-1 ${getStatusColor(perf.status)}`}>
                                {getStatusIcon(perf.status)}
                                {perf.status.toUpperCase()}
                              </div>
                              <div className="ml-auto text-2xl font-bold text-foreground">
                                {perf.score}
                              </div>
                            </div>

                            {/* Metrics Bars */}
                            <div className="grid grid-cols-3 gap-3">
                              <div>
                                <div className="flex items-center justify-between mb-1">
                                  <span className="text-xs text-muted-foreground">CPU</span>
                                  <span className="text-xs font-medium text-foreground">{perf.cpu.toFixed(1)}%</span>
                                </div>
                                <div className="h-1.5 bg-border rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-accent transition-all"
                                    style={{ width: `${perf.cpu}%` }}
                                  />
                                </div>
                              </div>
                              <div>
                                <div className="flex items-center justify-between mb-1">
                                  <span className="text-xs text-muted-foreground">Memory</span>
                                  <span className="text-xs font-medium text-foreground">{perf.memory.toFixed(1)}%</span>
                                </div>
                                <div className="h-1.5 bg-border rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-primary transition-all"
                                    style={{ width: `${perf.memory}%` }}
                                  />
                                </div>
                              </div>
                              <div>
                                <div className="flex items-center justify-between mb-1">
                                  <span className="text-xs text-muted-foreground">Disk</span>
                                  <span className="text-xs font-medium text-foreground">{perf.disk.toFixed(1)}%</span>
                                </div>
                                <div className="h-1.5 bg-border rounded-full overflow-hidden">
                                  <div
                                    className="h-full bg-emerald-400 transition-all"
                                    style={{ width: `${perf.disk}%` }}
                                  />
                                </div>
                              </div>
                            </div>
                          </div>
                        </div>
                      </div>
                    </Link>
                  ))}
                </div>
              )}
            </div>
          </>
        )}
      </main>
    </div>
  );
}

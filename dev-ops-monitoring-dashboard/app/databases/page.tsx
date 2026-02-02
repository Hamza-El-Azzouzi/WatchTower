'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import Link from 'next/link';
import { Database, Activity, HardDrive, Lock, TrendingUp } from 'lucide-react';
import { getAgents, getLatestMetrics } from '@/lib/api';
import { formatBytes, extractMetric } from '@/lib/metrics-utils';
import { Agent, LatestMetrics, DatabaseAgent, Metric } from '@/types';
import { useMetricsWebSocket } from '@/hooks/useWebSocket';
import { WsMetricMessage, WsAgentSnapshot, WsMetricSnapshot } from '@/lib/websocket';

export default function DatabasesPage() {
  const [dbAgents, setDbAgents] = useState<DatabaseAgent[]>([]);
  const [loading, setLoading] = useState(true);
  const dbAgentsRef = useRef<DatabaseAgent[]>([]);

  // Keep ref in sync with state
  useEffect(() => {
    dbAgentsRef.current = dbAgents;
  }, [dbAgents]);

  // Handle initial state from WebSocket
  const handleInitialState = useCallback((agents: WsAgentSnapshot[], metrics: WsMetricSnapshot[]) => {
    console.log('[DatabasesPage] Received initial state:', agents.length, 'agents,', metrics.length, 'metrics');
    
    // Group metrics by agent and filter for db_ metrics
    const agentMetricsMap = new Map<string, Metric[]>();
    const agentInfoMap = new Map<string, WsAgentSnapshot>();
    
    agents.forEach(agent => {
      agentInfoMap.set(agent.id, agent);
    });
    
    metrics.forEach(m => {
      if (m.metric_name.startsWith('db_')) {
        const existing = agentMetricsMap.get(m.agent_id) || [];
        existing.push({
          name: m.metric_name,
          value: m.latest_value,
          timestamp: m.timestamp,
        });
        agentMetricsMap.set(m.agent_id, existing);
      }
    });
    
    // Build DatabaseAgent array for agents with db_ metrics
    const dbAgentsList: DatabaseAgent[] = [];
    agentMetricsMap.forEach((agentMetrics, agentId) => {
      const agentInfo = agentInfoMap.get(agentId);
      if (agentInfo && agentMetrics.length > 0) {
        dbAgentsList.push({
          agent: {
            id: agentInfo.id,
            name: agentInfo.name,
            status: agentInfo.status as 'Healthy' | 'Degraded' | 'Unreachable',
            last_seen: agentInfo.last_seen,
          },
          metrics: {
            agent_id: agentId,
            metrics: agentMetrics,
            timestamp: agentMetrics[0]?.timestamp,
          },
        });
      }
    });
    
    setDbAgents(dbAgentsList);
    setLoading(false);
  }, []);

  // WebSocket handler for real-time metric updates
  const handleMetricUpdate = useCallback((metricMessage: WsMetricMessage) => {
    const agentId = metricMessage.agent_id;
    const metricName = metricMessage.metric_name;
    const metricValue = metricMessage.value;
    const timestamp = metricMessage.timestamp;

    // Only process db_ metrics
    if (!metricName.startsWith('db_')) {
      return;
    }

    setDbAgents(prev => {
      // Check if this agent already exists
      const existingAgentIndex = prev.findIndex(da => da.agent.id === agentId);
      
      if (existingAgentIndex >= 0) {
        // Update existing agent's metrics
        return prev.map(dbAgent => {
          if (dbAgent.agent.id !== agentId) return dbAgent;
          
          const existingMetrics = dbAgent.metrics?.metrics || [];
          const metricIndex = existingMetrics.findIndex(m => m.name === metricName);
          
          let updatedMetrics: Metric[];
          if (metricIndex >= 0) {
            // Update existing metric
            updatedMetrics = existingMetrics.map(m =>
              m.name === metricName ? { ...m, value: metricValue, timestamp } : m
            );
          } else {
            // Add new metric
            updatedMetrics = [...existingMetrics, { name: metricName, value: metricValue, timestamp }];
          }
          
          return {
            ...dbAgent,
            metrics: {
              agent_id: agentId,
              metrics: updatedMetrics,
              timestamp,
            },
          };
        });
      }
      
      // Agent doesn't exist yet - we'll need to add it when we get agent info
      // For now, just return prev (the initial state should have agent info)
      return prev;
    });
  }, []);

  // Connect to WebSocket with both handlers
  useMetricsWebSocket({
    onMetric: handleMetricUpdate,
    onInitialState: handleInitialState,
  });

  // Fallback: fetch data via HTTP if WebSocket doesn't provide initial state quickly
  useEffect(() => {
    const timeout = setTimeout(async () => {
      if (loading && dbAgents.length === 0) {
        console.log('[DatabasesPage] Fallback: fetching via HTTP');
        try {
          const agents = await getAgents();
          
          const agentsWithMetrics = await Promise.all(
            agents.map(async (agent) => {
              try {
                const metrics = await getLatestMetrics(agent.id);
                const hasDbMetrics = metrics.metrics.some(m => m.name.startsWith('db_'));
                return hasDbMetrics ? { agent, metrics } : null;
              } catch {
                return null;
              }
            })
          );

          const filteredAgents = agentsWithMetrics.filter((a): a is NonNullable<typeof a> => a !== null) as DatabaseAgent[];
          setDbAgents(filteredAgents);
        } catch (error) {
          console.error('Failed to fetch database agents:', error);
        } finally {
          setLoading(false);
        }
      }
    }, 2000); // Wait 2 seconds for WebSocket before falling back to HTTP

    return () => clearTimeout(timeout);
  }, [loading, dbAgents.length]);

  if (loading) {
    return (
      <div className="p-8">
        <div className="animate-pulse">
          <div className="h-8 bg-gray-700 rounded w-1/4 mb-6"></div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[1, 2, 3].map(i => (
              <div key={i} className="h-64 bg-gray-800 rounded-xl"></div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (dbAgents.length === 0) {
    return (
      <div className="p-8">
        <h1 className="text-3xl font-bold mb-6">Database Monitoring</h1>
        <div className="glass-morphism rounded-xl p-12 text-center">
          <Database className="w-16 h-16 text-muted-foreground mx-auto mb-4" />
          <h2 className="text-xl font-semibold mb-2">No Database Agents Found</h2>
          <p className="text-muted-foreground mb-4">
            Configure agents with database monitoring to see metrics here.
          </p>
          <Link 
            href="/docs/database-monitoring" 
            className="text-primary hover:underline"
          >
            Learn how to set up database monitoring →
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-6">
        <h1 className="text-3xl font-bold mb-2">Database Monitoring</h1>
        <p className="text-muted-foreground">
          Monitor database connections, queries, and performance metrics
        </p>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {dbAgents.map(({ agent, metrics }) => (
          <DatabaseCard key={agent.id} agent={agent} metrics={metrics} />
        ))}
      </div>
    </div>
  );
}

function DatabaseCard({ agent, metrics }: { agent: Agent; metrics: LatestMetrics | null }) {
  if (!metrics) return null;

  const connectionsActive = extractMetric(metrics.metrics, 'db_connections_active');
  const connectionsIdle = extractMetric(metrics.metrics, 'db_connections_idle');
  const connectionsMax = extractMetric(metrics.metrics, 'db_connections_max');
  const cacheHitRatio = extractMetric(metrics.metrics, 'db_cache_hit_ratio');
  const slowQueries = extractMetric(metrics.metrics, 'db_slow_queries');
  const dbSize = extractMetric(metrics.metrics, 'db_database_size_bytes');
  const locksWaiting = extractMetric(metrics.metrics, 'db_locks_waiting');

  const connectionUsage = connectionsMax > 0 
    ? ((connectionsActive + connectionsIdle) / connectionsMax * 100).toFixed(1)
    : 0;

  return (
    <Link href={`/databases/${encodeURIComponent(agent.id)}`}>
      <div className="glass-morphism rounded-xl p-6 cursor-pointer hover:border-primary/50 transition-all group">
        <div className="flex items-start justify-between mb-4">
          <div>
            <h3 className="text-lg font-bold group-hover:text-primary transition-colors">
              {agent.name}
            </h3>
            <p className="text-xs text-muted-foreground mt-1">
              Database Server
            </p>
          </div>
          <Database className="w-5 h-5 text-primary" />
        </div>

        {/* Connections */}
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2 text-sm">
              <Activity className="w-4 h-4 text-blue-400" />
              <span>Connections</span>
            </div>
            <span className="text-sm font-mono">{connectionUsage}%</span>
          </div>
          <div className="flex gap-2 text-xs text-muted-foreground">
            <span>Active: {connectionsActive}</span>
            <span>•</span>
            <span>Idle: {connectionsIdle}</span>
            <span>•</span>
            <span>Max: {connectionsMax}</span>
          </div>
          <div className="h-2 bg-gray-700 rounded-full overflow-hidden mt-2">
            <div 
              className="h-full bg-blue-500 transition-all"
              style={{ width: `${Math.min(parseFloat(connectionUsage.toString()), 100)}%` }}
            />
          </div>
        </div>

        {/* Cache Hit Ratio */}
        <div className="mb-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2 text-sm">
              <TrendingUp className="w-4 h-4 text-green-400" />
              <span>Cache Hit Ratio</span>
            </div>
            <span className="text-sm font-mono">{cacheHitRatio.toFixed(1)}%</span>
          </div>
          <div className="h-2 bg-gray-700 rounded-full overflow-hidden mt-2">
            <div 
              className="h-full bg-green-500 transition-all"
              style={{ width: `${Math.min(cacheHitRatio, 100)}%` }}
            />
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 gap-4 pt-4 border-t border-gray-700">
          <div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
              <HardDrive className="w-3 h-3" />
              <span>DB Size</span>
            </div>
            <p className="text-sm font-mono">{formatBytes(dbSize)}</p>
          </div>
          <div>
            <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
              <Lock className="w-3 h-3" />
              <span>Locks</span>
            </div>
            <p className="text-sm font-mono">{locksWaiting}</p>
          </div>
          <div className="col-span-2">
            <div className="text-xs text-muted-foreground mb-1">Slow Queries</div>
            <p className="text-sm font-mono">{slowQueries}</p>
          </div>
        </div>
      </div>
    </Link>
  );
}

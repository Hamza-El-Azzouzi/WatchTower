'use client';

import { useEffect, useState, useCallback } from 'react';
import PageHeader from '@/components/PageHeader';
import HeroBanner from '@/components/HeroBanner';
import StatsCards from '@/components/StatsCards';
import ServerCard from '@/components/ServerCard';
import SystemHealthOverview from '@/components/SystemHealthOverview';
import { getAgents } from '@/lib/api';
import { Agent } from '@/types';
import { AlertCircle, Loader, Wifi, WifiOff } from 'lucide-react';
import { useMetricsWebSocket } from '@/hooks/useWebSocket';
import { WsMetricMessage } from '@/lib/websocket';

export default function OverviewPage() {
  const [agents, setAgents] = useState<Agent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState(new Date());

  // Log component mount
  useEffect(() => {
    console.log('[OverviewPage] Component mounted');
    return () => console.log('[OverviewPage] Component unmounted');
  }, []);

  // Subscribe to WebSocket for real-time metrics
  const handleMetricUpdate = useCallback((metricMessage: WsMetricMessage) => {
    console.log('[OverviewPage] Received metric update:', metricMessage);
    setLastUpdated(new Date());
    // Update agent metrics in real-time as they come in via WebSocket
    setAgents(prev => 
      prev.map(agent => 
        agent.agent_id === metricMessage.Metric.agent_id
          ? { ...agent, last_seen: metricMessage.Metric.timestamp }
          : agent
      )
    );
  }, []);

  const { isConnected: wsConnected } = useMetricsWebSocket(handleMetricUpdate);
  
  // Log connection state changes
  useEffect(() => {
    console.log('[OverviewPage] WebSocket connected:', wsConnected);
  }, [wsConnected]);

  // Initial load of agents (one-time HTTP fetch)
  useEffect(() => {
    const fetchAgents = async () => {
      try {
        const data = await getAgents();
        setAgents(data);
        setLastUpdated(new Date());
        setError(null);
      } catch {
        setError('Failed to load servers. Make sure the API is running at http://localhost:8080');
      } finally {
        setLoading(false);
      }
    };

    fetchAgents();
  }, []);

  return (
    <div className="min-h-screen bg-background">
      <PageHeader />

      <main className="max-w-7xl mx-auto px-8 py-8">
        <HeroBanner />
        
        {error && (
          <div className="mb-8 glass-morphism rounded-xl border border-red-500/30 bg-red-500/10 p-4 flex items-start gap-3 animate-slide-up">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <h3 className="font-semibold text-red-300 mb-1">Connection Error</h3>
              <p className="text-sm text-red-200">{error}</p>
            </div>
          </div>
        )}

        {loading ? (
          <div className="flex flex-col items-center justify-center py-12">
            <Loader className="w-8 h-8 text-blue-400 animate-spin mb-4" />
            <p className="text-muted-foreground">Loading servers...</p>
          </div>
        ) : (
          <>
            <StatsCards agents={agents} lastUpdated={lastUpdated} />

            {agents.length === 0 ? (
              <div className="glass-morphism rounded-xl border border-border p-12 text-center animate-slide-up">
                <p className="text-muted-foreground mb-2">No servers found</p>
                <p className="text-sm text-muted-foreground">Make sure the API is running and has agents registered.</p>
              </div>
            ) : (
              <>
                {/* System Health Overview Section */}
                <div className="mb-12">
                  <SystemHealthOverview agents={agents} />
                </div>

                {/* Server Cards Grid */}
                <h2 className="text-2xl font-bold text-foreground mb-6">All Servers</h2>
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
                  {agents.map(agent => (
                    <ServerCard key={agent.id} agent={agent} />
                  ))}
                </div>
              </>
            )}

            <div className="mt-12 pt-8 border-t border-border flex items-center justify-between">
              <div className="text-xs text-muted-foreground flex items-center gap-2">
                {wsConnected ? (
                  <>
                    <Wifi className="w-3 h-3 text-emerald-500" />
                    <span className="text-emerald-500">WebSocket Connected</span>
                  </>
                ) : (
                  <>
                    <WifiOff className="w-3 h-3 text-amber-500" />
                    <span className="text-amber-500">WebSocket Disconnected</span>
                  </>
                )}
                <span className="mx-2">•</span>
                Last updated: {lastUpdated.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
              </div>
              <div className="flex items-center gap-3">
                <div className="text-right">
                  <div className="text-xs text-muted-foreground">Monitoring Status</div>
                  <div className="text-sm font-medium text-emerald-400">All Systems Normal</div>
                </div>
              </div>
            </div>
          </>
        )}
      </main>
    </div>
  );
}

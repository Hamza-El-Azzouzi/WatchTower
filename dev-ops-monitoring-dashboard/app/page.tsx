'use client';

import { useEffect } from 'react';
import PageHeader from '@/components/PageHeader';
import HeroBanner from '@/components/HeroBanner';
import StatsCards from '@/components/StatsCards';
import ServerCard from '@/components/ServerCard';
import SystemHealthOverview from '@/components/SystemHealthOverview';
import ConnectionStatus from '@/components/ConnectionStatus';
import { AlertCircle, Loader, Wifi, WifiOff } from 'lucide-react';
import { useMetricsContext } from '@/contexts/MetricsContext';

export default function OverviewPage() {
  // Use the shared WebSocket context
  const { 
    isConnected, 
    connectionState, 
    initialStateReceived, 
    agents, 
    agentMetrics, 
    lastUpdated 
  } = useMetricsContext();

  const loading = !initialStateReceived;

  // Log component mount
  useEffect(() => {
    console.log('[OverviewPage] Component mounted');
    return () => console.log('[OverviewPage] Component unmounted');
  }, []);

  // Log connection state changes
  useEffect(() => {
    console.log('[OverviewPage] WebSocket state:', connectionState);
  }, [connectionState]);

  return (
    <div className="min-h-screen bg-background">
      <PageHeader />

      <main className="max-w-7xl mx-auto px-8 py-8">
        {/* Global Connection Status */}
        <div className="flex justify-end mb-4">
          <ConnectionStatus state={connectionState} />
        </div>
        
        <HeroBanner />
        
        {connectionState === 'disconnected' && (
          <div className="mb-8 glass-morphism rounded-xl border border-red-500/30 bg-red-500/10 p-4 flex items-start gap-3 animate-slide-up">
            <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
            <div>
              <h3 className="font-semibold text-red-300 mb-1">Connection Error</h3>
              <p className="text-sm text-red-200">WebSocket disconnected. Attempting to reconnect...</p>
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
                {isConnected ? (
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

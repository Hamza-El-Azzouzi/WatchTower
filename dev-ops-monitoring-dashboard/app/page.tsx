'use client';

import PageHeader from '@/components/PageHeader';
import HeroBanner from '@/components/HeroBanner';
import StatsCards from '@/components/StatsCards';
import ServerCard from '@/components/ServerCard';
import SystemHealthOverview from '@/components/SystemHealthOverview';
import { AlertCircle, Loader, Server } from 'lucide-react';
import { useMetricsContext } from '@/contexts/MetricsContext';

export default function OverviewPage() {
  // Use the shared WebSocket context
  const { 
    isConnected, 
    connectionState, 
    initialStateReceived, 
    agents, 
    lastUpdated 
  } = useMetricsContext();

  const loading = !initialStateReceived;

  return (
    <div className="min-h-screen">
      <PageHeader />

      <main className="page-shell">
        <HeroBanner />
        
        {connectionState === 'disconnected' && (
          <div role="alert" className="mb-6 flex items-start gap-3 rounded-2xl border border-rose-400/20 bg-rose-400/8 p-4 animate-slide-up">
            <AlertCircle className="mt-0.5 h-5 w-5 shrink-0 text-rose-300" />
            <div>
              <h3 className="font-semibold text-rose-200">Live connection interrupted</h3>
              <p className="mt-1 text-sm text-rose-200/70">Showing the latest received values while WatchTower reconnects.</p>
            </div>
          </div>
        )}

        {loading ? (
          <div className="surface-panel flex min-h-72 flex-col items-center justify-center rounded-3xl">
            <Loader className="mb-4 h-7 w-7 animate-spin text-cyan-300" />
            <p className="text-sm text-muted-foreground">Synchronizing fleet telemetry…</p>
          </div>
        ) : (
          <>
            <StatsCards agents={agents} lastUpdated={lastUpdated} />

            {agents.length === 0 ? (
              <div className="surface-panel rounded-3xl p-12 text-center animate-slide-up">
                <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-2xl bg-cyan-400/10 text-cyan-300"><Server className="h-5 w-5" /></div>
                <p className="font-semibold text-slate-200">Your fleet is ready for its first node</p>
                <p className="mx-auto mt-2 max-w-md text-sm text-muted-foreground">Connect an agent with an API key and it will appear here automatically.</p>
              </div>
            ) : (
              <>
                {/* System Health Overview Section */}
                <div className="mb-10">
                  <SystemHealthOverview agents={agents} />
                </div>

                {/* Server Cards Grid */}
                <div id="servers" className="mb-5 flex items-end justify-between scroll-mt-28"><div><p className="eyebrow">Inventory</p><h2 className="mt-1 text-xl font-semibold tracking-tight text-white">All servers</h2></div><p className="text-xs text-slate-500">{agents.length} nodes</p></div>
                <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
                  {agents.map(agent => (
                    <ServerCard key={agent.id} agent={agent} />
                  ))}
                </div>
              </>
            )}

            <div className="mt-10 flex flex-col gap-2 border-t border-white/6 py-6 text-xs text-slate-600 sm:flex-row sm:items-center sm:justify-between">
              <p>{isConnected ? 'Live telemetry connected' : 'Telemetry reconnecting'} · Last update {lastUpdated.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}</p>
              <p>WatchTower operational workspace</p>
            </div>
          </>
        )}
      </main>
    </div>
  );
}

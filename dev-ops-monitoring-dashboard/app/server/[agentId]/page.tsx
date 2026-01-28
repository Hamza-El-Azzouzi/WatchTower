'use client';

import { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import Link from 'next/link';
import { ArrowLeft, AlertCircle, Loader } from 'lucide-react';
import StatusBadge from '@/components/StatusBadge';
import MetricsSection from '@/components/MetricsSection';
import ChartsSection from '@/components/ChartsSection';
import AlertThresholdChart from '@/components/AlertThresholdChart';
import { getAgents, getLatestMetrics } from '@/lib/api';
import { formatRelativeTime } from '@/lib/metrics-utils';
import { Agent, LatestMetrics } from '@/types';

export default function ServerDetailPage() {
  const params = useParams();
  const router = useRouter();
  const agentId = decodeURIComponent(params.agentId as string);

  const [agent, setAgent] = useState<Agent | null>(null);
  const [metrics, setMetrics] = useState<LatestMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isInitialLoad = true;

    const fetchData = async () => {
      try {
        // Only show loading state on initial load, not on refreshes
        if (isInitialLoad) {
          setLoading(true);
        }
        
        const agents = await getAgents();
        const foundAgent = agents.find(a => a.id === agentId);

        if (!foundAgent) {
          setError('Server not found');
          return;
        }

        setAgent(foundAgent);

        const metricsData = await getLatestMetrics(agentId);
        setMetrics(metricsData);
        setError(null);
      } catch {
        setError('Failed to load server details');
      } finally {
        if (isInitialLoad) {
          setLoading(false);
          isInitialLoad = false;
        }
      }
    };

    if (agentId) {
      fetchData();

      // Poll every 10 seconds
      const interval = setInterval(fetchData, 10000);
      return () => clearInterval(interval);
    }
  }, [agentId]);

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <div className="border-b border-border glass-morphism sticky top-0 z-10 ml-64">
        <div className="max-w-full px-8 py-6">
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
      <main className="max-w-full px-8 py-8">
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

            <h2 className="text-2xl font-bold text-foreground mb-6 mt-12">Alert Thresholds</h2>
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-12">
              <AlertThresholdChart
                agentId={agentId}
                metric="cpu_usage"
                title="CPU Usage with Thresholds"
                warningThreshold={70}
                criticalThreshold={90}
                limit={100}
              />
              <AlertThresholdChart
                agentId={agentId}
                metric="memory_usage"
                title="Memory Usage with Thresholds"
                warningThreshold={75}
                criticalThreshold={90}
                limit={100}
              />
              <AlertThresholdChart
                agentId={agentId}
                metric="disk_usage"
                title="Disk Usage with Thresholds"
                warningThreshold={80}
                criticalThreshold={95}
                limit={100}
              />
            </div>

            <h2 className="text-2xl font-bold text-foreground mb-6 mt-12">Historical Charts</h2>
            <ChartsSection agentId={agentId} />
          </>
        ) : null}
      </main>
    </div>
  );
}

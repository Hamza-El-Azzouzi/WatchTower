'use client';

import PageHeader from '@/components/PageHeader';
import { Zap, Gauge, Activity, TrendingUp } from 'lucide-react';

export default function PerformancePage() {
  return (
    <div className="min-h-screen bg-background">
      <PageHeader />

      <main className="max-w-7xl mx-auto px-8 py-8">
        <div className="glass-morphism rounded-xl border border-border p-12 text-center">
          <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-accent/20 mb-6">
            <Zap className="w-10 h-10 text-accent" />
          </div>
          <h1 className="text-3xl font-bold text-foreground mb-4">Performance Analytics</h1>
          <p className="text-muted-foreground max-w-2xl mx-auto mb-6">
            Advanced performance monitoring coming in Phase 7. Features will include:
          </p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 max-w-4xl mx-auto mt-8">
            <div className="glass-morphism rounded-lg border border-border p-6">
              <Gauge className="w-8 h-8 text-accent mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Response Times</h3>
              <p className="text-sm text-muted-foreground">Track API and application response times</p>
            </div>
            <div className="glass-morphism rounded-lg border border-border p-6">
              <Activity className="w-8 h-8 text-primary mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Request Tracing</h3>
              <p className="text-sm text-muted-foreground">Distributed tracing across services</p>
            </div>
            <div className="glass-morphism rounded-lg border border-border p-6">
              <TrendingUp className="w-8 h-8 text-emerald-400 mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Bottleneck Detection</h3>
              <p className="text-sm text-muted-foreground">Identify performance bottlenecks automatically</p>
            </div>
          </div>
          <p className="text-sm text-muted-foreground mt-8">
            Phase 7 will add comprehensive performance profiling!
          </p>
        </div>
      </main>
    </div>
  );
}

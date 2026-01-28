'use client';

import PageHeader from '@/components/PageHeader';
import { Database, Clock, TrendingUp } from 'lucide-react';

export default function DatabasePage() {
  return (
    <div className="min-h-screen bg-background">
      <PageHeader />

      <main className="max-w-7xl mx-auto px-8 py-8">
        <div className="glass-morphism rounded-xl border border-border p-12 text-center">
          <div className="inline-flex items-center justify-center w-20 h-20 rounded-full bg-primary/20 mb-6">
            <Database className="w-10 h-10 text-primary" />
          </div>
          <h1 className="text-3xl font-bold text-foreground mb-4">Database Monitoring</h1>
          <p className="text-muted-foreground max-w-2xl mx-auto mb-6">
            Database monitoring features are coming in Phase 5. This will include:
          </p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4 max-w-4xl mx-auto mt-8">
            <div className="glass-morphism rounded-lg border border-border p-6">
              <Clock className="w-8 h-8 text-primary mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Query Performance</h3>
              <p className="text-sm text-muted-foreground">Track slow queries and execution times</p>
            </div>
            <div className="glass-morphism rounded-lg border border-border p-6">
              <Database className="w-8 h-8 text-accent mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Connection Pools</h3>
              <p className="text-sm text-muted-foreground">Monitor active connections and pool usage</p>
            </div>
            <div className="glass-morphism rounded-lg border border-border p-6">
              <TrendingUp className="w-8 h-8 text-emerald-400 mb-3 mx-auto" />
              <h3 className="font-semibold text-foreground mb-2">Storage Metrics</h3>
              <p className="text-sm text-muted-foreground">Track database size and growth trends</p>
            </div>
          </div>
          <p className="text-sm text-muted-foreground mt-8">
            Stay tuned for Phase 5 implementation!
          </p>
        </div>
      </main>
    </div>
  );
}

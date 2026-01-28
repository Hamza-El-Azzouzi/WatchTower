'use client';

import { Server, CheckCircle2, AlertCircle, XCircle, Clock } from 'lucide-react';
import { Agent } from '@/types';

interface StatsCardsProps {
  agents: Agent[];
  lastUpdated: Date;
}

export default function StatsCards({ agents, lastUpdated }: StatsCardsProps) {
  const totalServers = agents.length;
  const healthyServers = agents.filter(a => a.status === 'Healthy').length;
  const degradedServers = agents.filter(a => a.status === 'Degraded').length;
  const unreachableServers = agents.filter(a => a.status === 'Unreachable').length;

  const secondsAgo = Math.floor((Date.now() - lastUpdated.getTime()) / 1000);
  const lastUpdatedText = secondsAgo < 60 ? `${secondsAgo}s ago` : `${Math.floor(secondsAgo / 60)}m ago`;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <div className="glass-morphism rounded-xl p-6 hover:shadow-lg transition-smooth group animate-slide-up">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-muted-foreground mb-2">Total Servers</p>
            <p className="text-4xl font-bold bg-gradient-to-r from-primary to-accent bg-clip-text text-transparent">{totalServers}</p>
          </div>
          <div className="w-12 h-12 rounded-lg bg-primary/20 flex items-center justify-center group-hover:scale-110 transition-smooth">
            <Server className="w-6 h-6 text-primary" />
          </div>
        </div>
        <div className="mt-4 h-1 w-full bg-gradient-to-r from-primary/30 to-transparent rounded-full" />
      </div>

      <div className="glass-morphism rounded-xl p-6 hover:shadow-lg transition-smooth group animate-slide-up" style={{ animationDelay: '50ms' }}>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-emerald-300 mb-2">Healthy Servers</p>
            <p className="text-4xl font-bold text-emerald-400">{healthyServers}</p>
          </div>
          <div className="w-12 h-12 rounded-lg bg-emerald-500/20 flex items-center justify-center group-hover:scale-110 transition-smooth">
            <CheckCircle2 className="w-6 h-6 text-emerald-400" />
          </div>
        </div>
        <div className="mt-4 h-1 w-full bg-gradient-to-r from-emerald-500/30 to-transparent rounded-full" />
      </div>

      <div className="glass-morphism rounded-xl p-6 hover:shadow-lg transition-smooth group animate-slide-up" style={{ animationDelay: '100ms' }}>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-amber-300 mb-2">Degraded Servers</p>
            <p className="text-4xl font-bold text-amber-400">{degradedServers}</p>
          </div>
          <div className="w-12 h-12 rounded-lg bg-amber-500/20 flex items-center justify-center group-hover:scale-110 transition-smooth">
            <AlertCircle className="w-6 h-6 text-amber-400" />
          </div>
        </div>
        <div className="mt-4 h-1 w-full bg-gradient-to-r from-amber-500/30 to-transparent rounded-full" />
      </div>

      <div className="glass-morphism rounded-xl p-6 hover:shadow-lg transition-smooth group animate-slide-up" style={{ animationDelay: '150ms' }}>
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm text-red-300 mb-2">Unreachable Servers</p>
            <p className="text-4xl font-bold text-red-400">{unreachableServers}</p>
          </div>
          <div className="w-12 h-12 rounded-lg bg-red-500/20 flex items-center justify-center group-hover:scale-110 transition-smooth">
            <XCircle className="w-6 h-6 text-red-400" />
          </div>
        </div>
        <div className="mt-4 h-1 w-full bg-gradient-to-r from-red-500/30 to-transparent rounded-full" />
      </div>
    </div>
  );
}

'use client';

import Link from 'next/link';
import { ArrowUpRight, Cpu, HardDrive, MemoryStick, Server } from 'lucide-react';
import StatusBadge from './StatusBadge';
import { formatRelativeTime, extractMetric } from '@/lib/metrics-utils';
import { Agent } from '@/types';
import { useMetricsContext } from '@/contexts/MetricsContext';

export default function ServerCard({ agent }: { agent: Agent }) {
  const { agentMetrics, initialStateReceived } = useMetricsContext();
  const metrics = agentMetrics[agent.id]?.metrics ?? [];
  const loading = !initialStateReceived;
  const values = [
    { label: 'CPU', value: extractMetric(metrics, 'cpu_usage'), icon: Cpu },
    { label: 'Memory', value: extractMetric(metrics, 'memory_usage'), icon: MemoryStick },
    { label: 'Disk', value: extractMetric(metrics, 'disk_usage'), icon: HardDrive },
  ];
  const worst = Math.max(...values.map(item => item.value));
  const pressure = worst >= 90 ? 'Critical pressure' : worst >= 75 ? 'Elevated load' : 'Within thresholds';

  return (
    <Link href={`/server/${encodeURIComponent(agent.id)}`} className="group block rounded-[20px] focus-visible:outline-none">
      <article className="surface-panel relative h-full overflow-hidden rounded-[20px] p-5 transition duration-200 group-hover:-translate-y-0.5 group-hover:border-cyan-300/25 group-hover:shadow-[0_24px_70px_rgba(0,0,0,.28)]">
        <div className="absolute inset-x-0 top-0 h-px bg-gradient-to-r from-transparent via-cyan-300/30 to-transparent opacity-0 transition group-hover:opacity-100" />
        <div className="flex items-start justify-between gap-4">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-white/[.045] text-slate-300 ring-1 ring-white/8"><Server className="h-[18px] w-[18px]" /></div>
            <div className="min-w-0"><h3 className="truncate text-[15px] font-semibold text-slate-100 transition group-hover:text-cyan-200">{agent.name}</h3><p className="mt-0.5 truncate font-mono text-[10px] text-slate-600">{agent.id}</p></div>
          </div>
          <StatusBadge status={agent.status} size="sm" />
        </div>

        <div className="my-5 grid grid-cols-3 gap-3">
          {values.map(item => <Resource key={item.label} {...item} loading={loading} />)}
        </div>

        <div className="flex items-center justify-between border-t border-white/6 pt-4">
          <div><p className={`text-xs font-medium ${worst >= 90 ? 'text-rose-300' : worst >= 75 ? 'text-amber-300' : 'text-lime-300'}`}>{pressure}</p><p className="mt-0.5 text-[10px] text-slate-600">Seen {formatRelativeTime(agent.last_seen)}</p></div>
          <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-white/[.035] text-slate-500 transition group-hover:bg-cyan-400/10 group-hover:text-cyan-300"><ArrowUpRight className="h-4 w-4" /></span>
        </div>
      </article>
    </Link>
  );
}

function Resource({ label, value, icon: Icon, loading }: { label: string; value: number; icon: typeof Cpu; loading: boolean }) {
  const bar = value >= 90 ? 'bg-rose-400' : value >= 75 ? 'bg-amber-400' : 'bg-cyan-300';
  return <div><div className="mb-2 flex items-center gap-1.5 text-[10px] font-medium uppercase tracking-[.1em] text-slate-500"><Icon className="h-3 w-3" />{label}</div><p className="text-lg font-semibold tabular-nums text-slate-100">{loading ? '—' : `${value.toFixed(0)}%`}</p><div className="metric-track mt-2"><div className={`h-full rounded-full ${bar} transition-[width] duration-500`} style={{ width: `${Math.min(value, 100)}%` }} /></div></div>
}

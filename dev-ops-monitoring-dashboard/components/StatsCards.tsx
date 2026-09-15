'use client';

import { Server, CheckCircle2, AlertTriangle, WifiOff } from 'lucide-react';
import { Agent } from '@/types';

export default function StatsCards({ agents, lastUpdated }: { agents: Agent[]; lastUpdated: Date }) {
  const items = [
    { label: 'Total nodes', value: agents.length, icon: Server, tone: 'cyan', note: 'Registered fleet' },
    { label: 'Operational', value: agents.filter(agent => agent.status === 'Healthy').length, icon: CheckCircle2, tone: 'lime', note: 'Reporting normally' },
    { label: 'Degraded', value: agents.filter(agent => agent.status === 'Degraded').length, icon: AlertTriangle, tone: 'amber', note: 'Needs observation' },
    { label: 'Unreachable', value: agents.filter(agent => agent.status === 'Unreachable').length, icon: WifiOff, tone: 'rose', note: 'Action required' },
  ] as const;
  const secondsAgo = Math.max(0, Math.floor((Date.now() - lastUpdated.getTime()) / 1000));

  return (
    <section className="mb-8">
      <div className="mb-4 flex items-end justify-between gap-4"><div><p className="eyebrow">Fleet status</p><h2 className="mt-1 text-xl font-semibold tracking-tight text-white">Operational snapshot</h2></div><p className="hidden text-xs text-slate-500 sm:block">Updated {secondsAgo < 60 ? `${secondsAgo}s` : `${Math.floor(secondsAgo / 60)}m`} ago</p></div>
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        {items.map((item, index) => <Stat key={item.label} {...item} delay={index * 45} />)}
      </div>
    </section>
  );
}

const toneMap = {
  cyan: 'bg-cyan-400/10 text-cyan-300 ring-cyan-400/15',
  lime: 'bg-lime-400/10 text-lime-300 ring-lime-400/15',
  amber: 'bg-amber-400/10 text-amber-300 ring-amber-400/15',
  rose: 'bg-rose-400/10 text-rose-300 ring-rose-400/15',
};

function Stat({ label, value, icon: Icon, tone, note, delay }: { label: string; value: number; icon: typeof Server; tone: keyof typeof toneMap; note: string; delay: number }) {
  return <div className="surface-panel animate-slide-up rounded-2xl p-4 sm:p-5" style={{ animationDelay: `${delay}ms` }}><div className="flex items-start justify-between gap-3"><div><p className="text-xs font-medium text-slate-400">{label}</p><p className="mt-2 text-3xl font-semibold tracking-[-.04em] tabular-nums text-white sm:text-4xl">{value}</p></div><div className={`flex h-9 w-9 items-center justify-center rounded-xl ring-1 ${toneMap[tone]}`}><Icon className="h-[18px] w-[18px]" /></div></div><p className="mt-3 hidden text-[11px] text-slate-600 sm:block">{note}</p></div>
}

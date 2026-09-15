'use client';

import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip } from 'recharts';
import { Agent } from '@/types';
import { Activity, Cpu, HardDrive, MemoryStick } from 'lucide-react';
import { useMetricsContext } from '@/contexts/MetricsContext';

export default function SystemHealthOverview({ agents }: { agents: Agent[] }) {
  const { agentMetrics, initialStateReceived } = useMetricsContext();
  const valueOf = (agentId: string, name: string) => agentMetrics[agentId]?.metrics.find(metric => metric.name === name)?.value ?? 0;
  const average = (name: string) => agents.reduce((sum, agent) => sum + valueOf(agent.id, name), 0) / Math.max(agents.length, 1);
  const resources = [
    { name: 'CPU', value: average('cpu_usage'), icon: Cpu, color: '#22d3ee' },
    { name: 'Memory', value: average('memory_usage'), icon: MemoryStick, color: '#a78bfa' },
    { name: 'Disk', value: average('disk_usage'), icon: HardDrive, color: '#84cc16' },
  ];
  const healthScore = Math.round(resources.reduce((sum, resource) => sum + Math.max(0, 100 - resource.value), 0) / resources.length);
  const comparison = agents.slice(0, 10).map(agent => ({
    name: agent.name.length > 14 ? `${agent.name.slice(0, 13)}…` : agent.name,
    CPU: Number(valueOf(agent.id, 'cpu_usage').toFixed(1)),
    Memory: Number(valueOf(agent.id, 'memory_usage').toFixed(1)),
    Disk: Number(valueOf(agent.id, 'disk_usage').toFixed(1)),
  }));

  if (!initialStateReceived) return <div className="surface-panel h-80 animate-pulse rounded-3xl" />;

  return (
    <section>
      <div className="mb-5"><p className="eyebrow">Resource posture</p><h2 className="mt-1 text-xl font-semibold tracking-tight text-white">Fleet health</h2></div>
      <div className="grid gap-4 xl:grid-cols-[320px_1fr]">
        <div className="surface-panel rounded-[20px] p-5">
          <div className="flex items-center gap-2 text-sm font-medium text-slate-200"><Activity className="h-4 w-4 text-cyan-300" /> Health index</div>
          <div className="my-6 flex items-center gap-5">
            <div className="relative flex h-28 w-28 shrink-0 items-center justify-center rounded-full" style={{ background: `conic-gradient(${healthScore >= 80 ? '#84cc16' : healthScore >= 60 ? '#fbbf24' : '#fb7185'} ${healthScore * 3.6}deg, rgba(255,255,255,.055) 0deg)` }}><div className="flex h-[88px] w-[88px] flex-col items-center justify-center rounded-full bg-[#0d141e]"><span className="text-3xl font-semibold tracking-tight text-white">{healthScore}</span><span className="text-[9px] uppercase tracking-[.16em] text-slate-500">of 100</span></div></div>
            <div><p className="font-medium text-slate-100">{healthScore >= 80 ? 'Fleet is stable' : healthScore >= 60 ? 'Watch capacity' : 'Intervention needed'}</p><p className="mt-2 text-xs leading-5 text-slate-500">Weighted from current CPU, memory, and disk headroom.</p></div>
          </div>
          <div className="space-y-4 border-t border-white/6 pt-5">
            {resources.map(resource => <ResourceRow key={resource.name} {...resource} />)}
          </div>
        </div>

        <div className="surface-panel min-w-0 rounded-[20px] p-5">
          <div className="mb-5 flex items-center justify-between"><div><p className="text-sm font-medium text-slate-200">Node comparison</p><p className="mt-1 text-xs text-slate-500">Utilization by resource · latest sample</p></div><div className="hidden gap-3 text-[10px] text-slate-500 sm:flex"><Legend color="bg-cyan-300" label="CPU" /><Legend color="bg-violet-400" label="Memory" /><Legend color="bg-lime-400" label="Disk" /></div></div>
          <div className="h-[285px] w-full">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={comparison} barGap={2}>
                <CartesianGrid vertical={false} stroke="rgba(148,163,184,.08)" />
                <XAxis dataKey="name" axisLine={false} tickLine={false} tick={{ fill: '#64748b', fontSize: 10 }} dy={10} />
                <YAxis domain={[0, 100]} axisLine={false} tickLine={false} tick={{ fill: '#64748b', fontSize: 10 }} width={30} />
                <Tooltip cursor={{ fill: 'rgba(255,255,255,.025)' }} contentStyle={{ background: '#0b121b', border: '1px solid #203040', borderRadius: 12, color: '#e2e8f0', fontSize: 12 }} />
                <Bar dataKey="CPU" fill="#22d3ee" radius={[3, 3, 0, 0]} maxBarSize={16} />
                <Bar dataKey="Memory" fill="#a78bfa" radius={[3, 3, 0, 0]} maxBarSize={16} />
                <Bar dataKey="Disk" fill="#84cc16" radius={[3, 3, 0, 0]} maxBarSize={16} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </div>
      </div>
    </section>
  );
}

function ResourceRow({ name, value, icon: Icon, color }: { name: string; value: number; icon: typeof Cpu; color: string }) {
  return <div><div className="mb-2 flex items-center justify-between"><span className="flex items-center gap-2 text-xs text-slate-400"><Icon className="h-3.5 w-3.5" />{name}</span><span className="text-xs font-semibold tabular-nums text-slate-200">{value.toFixed(1)}%</span></div><div className="metric-track"><div className="h-full rounded-full" style={{ width: `${Math.min(value, 100)}%`, backgroundColor: color }} /></div></div>;
}

function Legend({ color, label }: { color: string; label: string }) {
  return <span className="flex items-center gap-1.5"><span className={`h-1.5 w-1.5 rounded-full ${color}`} />{label}</span>;
}

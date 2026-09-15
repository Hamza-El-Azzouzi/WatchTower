'use client'

import { Activity, ArrowUpRight, Cpu, MemoryStick, ShieldCheck } from 'lucide-react'
import Link from 'next/link'
import { useMetricsContext } from '@/contexts/MetricsContext'

export default function HeroBanner() {
  const { agents, agentMetrics } = useMetricsContext()
  const average = (name: string) => {
    const values = agents.flatMap(agent => agentMetrics[agent.id]?.metrics.find(metric => metric.name === name)?.value ?? [])
    return values.length ? values.reduce((sum, value) => sum + value, 0) / values.length : 0
  }
  const cpu = average('cpu_usage')
  const memory = average('memory_usage')
  const healthy = agents.filter(agent => agent.status === 'Healthy').length
  const health = agents.length ? Math.round((healthy / agents.length) * 100) : 100

  return (
    <section className="surface-panel relative mb-6 overflow-hidden rounded-[24px] p-6 sm:p-8 lg:p-10">
      <div className="pointer-events-none absolute inset-0 opacity-50" style={{ backgroundImage: 'linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.025) 1px, transparent 1px)', backgroundSize: '38px 38px' }} />
      <div className="pointer-events-none absolute -right-24 -top-32 h-80 w-80 rounded-full bg-cyan-400/10 blur-3xl" />
      <div className="relative grid gap-8 lg:grid-cols-[1.25fr_.75fr] lg:items-end">
        <div>
          <div className="mb-5 inline-flex items-center gap-2 rounded-full border border-lime-400/20 bg-lime-400/8 px-3 py-1.5 text-xs font-medium text-lime-300">
            <span className="relative flex h-2 w-2"><span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-lime-400 opacity-40" /><span className="relative h-2 w-2 rounded-full bg-lime-400" /></span>
            Fleet telemetry is live
          </div>
          <p className="eyebrow mb-3 text-cyan-300/70">Command center</p>
          <h2 className="max-w-2xl text-3xl font-semibold leading-[1.08] tracking-[-0.045em] text-white sm:text-4xl lg:text-5xl">
            See pressure before it becomes an incident.
          </h2>
          <p className="mt-4 max-w-xl text-sm leading-6 text-slate-400 sm:text-base">A focused view of fleet health, resource saturation, and systems that need your attention now.</p>
          <div className="mt-7 flex flex-wrap gap-3">
            <Link href="#servers" className="inline-flex items-center gap-2 rounded-xl bg-cyan-300 px-4 py-2.5 text-sm font-semibold text-[#061016] transition hover:bg-cyan-200">Inspect fleet <ArrowUpRight className="h-4 w-4" /></Link>
            <Link href="/alerts" className="inline-flex items-center gap-2 rounded-xl border border-white/10 bg-white/[.035] px-4 py-2.5 text-sm font-medium text-slate-200 transition hover:bg-white/[.07]">Review incidents</Link>
          </div>
        </div>
        <div className="grid grid-cols-3 gap-2 sm:gap-3">
          <Signal icon={Cpu} label="Avg CPU" value={`${cpu.toFixed(0)}%`} tone="cyan" />
          <Signal icon={MemoryStick} label="Memory" value={`${memory.toFixed(0)}%`} tone="violet" />
          <Signal icon={ShieldCheck} label="Health" value={`${health}%`} tone="lime" />
        </div>
      </div>
    </section>
  )
}

function Signal({ icon: Icon, label, value, tone }: { icon: typeof Activity; label: string; value: string; tone: 'cyan' | 'violet' | 'lime' }) {
  const tones = { cyan: 'text-cyan-300 bg-cyan-400/10', violet: 'text-violet-300 bg-violet-400/10', lime: 'text-lime-300 bg-lime-400/10' }
  return <div className="rounded-2xl border border-white/8 bg-[#080d15]/55 p-3.5 sm:p-4"><div className={`mb-5 flex h-8 w-8 items-center justify-center rounded-lg ${tones[tone]}`}><Icon className="h-4 w-4" /></div><p className="text-[10px] font-medium uppercase tracking-[.13em] text-slate-500">{label}</p><p className="mt-1 text-xl font-semibold tabular-nums text-white sm:text-2xl">{value}</p></div>
}

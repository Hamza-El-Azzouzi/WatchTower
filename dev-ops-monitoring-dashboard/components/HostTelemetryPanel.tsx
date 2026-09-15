'use client';

import {
  Activity,
  Box,
  Clock3,
  Cpu,
  DatabaseZap,
  Gauge,
  HardDrive,
  MemoryStick,
  Network,
  ServerCog,
  ShieldCheck,
} from 'lucide-react';
import type { ReactNode } from 'react';
import type { LatestMetrics } from '@/types';
import type { HostTelemetrySnapshot } from '@/lib/websocket';
import { extractMetric, formatBytes } from '@/lib/metrics-utils';

type Props = {
  telemetry: HostTelemetrySnapshot | null;
  metrics: LatestMetrics | null;
};

export default function HostTelemetryPanel({ telemetry, metrics }: Props) {
  const metric = (name: string) => metrics ? extractMetric(metrics.metrics, name) : 0;
  const cpuModes = [
    ['user', metric('cpu_user_percent')],
    ['system', metric('cpu_system_percent')],
    ['iowait', metric('cpu_iowait_percent')],
    ['steal', metric('cpu_steal_percent')],
  ] as const;
  const oomDelta = metric('oom_kills_delta');

  return (
    <section className="mt-12 space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.18em] text-cyan-300">
            <Gauge className="h-4 w-4" /> Host diagnostics
          </div>
          <h2 className="text-2xl font-bold text-foreground">Operating system telemetry</h2>
          <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
            Resource pressure, storage, interfaces, services, and containers in one read-only operations view.
          </p>
        </div>
        <div className="inline-flex w-fit items-center gap-2 rounded-full border border-emerald-400/20 bg-emerald-400/10 px-3 py-1.5 text-xs font-medium text-emerald-300">
          <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-300" />
          {telemetry ? `Updated ${new Date(telemetry.timestamp).toLocaleTimeString()}` : 'Awaiting agent sample'}
        </div>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        <SignalCard icon={<Clock3 />} label="Uptime & load" value={formatDuration(metric('uptime_seconds'))}>
          Load {metric('load_1').toFixed(2)} · {metric('load_5').toFixed(2)} · {metric('load_15').toFixed(2)}
        </SignalCard>
        <SignalCard icon={<Cpu />} label="CPU execution" value={`${metric('cpu_usage').toFixed(1)}%`}>
          <div className="flex flex-wrap gap-x-3 gap-y-1">
            {cpuModes.map(([name, value]) => <span key={name}>{name} {value.toFixed(1)}%</span>)}
          </div>
        </SignalCard>
        <SignalCard icon={<MemoryStick />} label="Memory headroom" value={formatBytes(metric('memory_available_bytes'))}>
          Available · {formatBytes(metric('memory_cached_bytes'))} cached
        </SignalCard>
        <SignalCard
          icon={<DatabaseZap />}
          label="Swap & OOM"
          value={`${formatRate(metric('swap_in_bytes_per_sec'))} in`}
          tone={oomDelta > 0 ? 'danger' : 'normal'}
        >
          {formatRate(metric('swap_out_bytes_per_sec'))} out · {metric('oom_kills_total').toFixed(0)} OOM kills
        </SignalCard>
        <SignalCard icon={<Network />} label="TCP sockets" value={metric('tcp_connections_total').toFixed(0)}>
          {metric('tcp_established').toFixed(0)} established · {metric('tcp_listen').toFixed(0)} listening · {metric('tcp_time_wait').toFixed(0)} time-wait
        </SignalCard>
        <SignalCard icon={<ShieldCheck />} label="Host signals" value={healthLabel(telemetry, oomDelta)} tone={healthLabel(telemetry, oomDelta) === 'Attention' ? 'danger' : 'normal'}>
          {metric('services_unhealthy').toFixed(0)} service issues · {metric('containers_unhealthy').toFixed(0)} container issues
        </SignalCard>
      </div>

      <div className="grid gap-5 xl:grid-cols-2">
        <DataPanel icon={<HardDrive />} title="Filesystems" count={telemetry?.mounts.length ?? 0} empty="No filesystem telemetry yet">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead><TableHead labels={['Mount', 'Space', 'Inodes', 'Read / write', 'IOPS', 'Latency']} /></thead>
            <tbody>
              {telemetry?.mounts.map(mount => (
                <tr key={`${mount.device}:${mount.mount_point}`} className="border-t border-border/70 hover:bg-white/[.025]">
                  <Cell><div className="font-medium text-foreground">{mount.mount_point}</div><div className="text-xs text-muted-foreground">{mount.device} · {mount.filesystem}</div></Cell>
                  <Cell><Usage value={mount.usage_percent} /> <div className="mt-1 text-xs text-muted-foreground">{formatBytes(mount.used_bytes)} / {formatBytes(mount.total_bytes)}</div></Cell>
                  <Cell><Usage value={mount.inode_usage_percent} /></Cell>
                  <Cell mono>{formatRate(mount.read_bytes_per_sec)} / {formatRate(mount.write_bytes_per_sec)}</Cell>
                  <Cell mono>{mount.read_iops.toFixed(1)} / {mount.write_iops.toFixed(1)}</Cell>
                  <Cell mono>{mount.average_latency_ms.toFixed(1)} ms</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        </DataPanel>

        <DataPanel icon={<Network />} title="Network interfaces" count={telemetry?.network_interfaces.length ?? 0} empty="No interface telemetry yet">
          <table className="w-full min-w-[680px] text-left text-sm">
            <thead><TableHead labels={['Interface', 'Receive', 'Transmit', 'Packets RX / TX', 'Errors', 'Dropped']} /></thead>
            <tbody>
              {telemetry?.network_interfaces.map(item => (
                <tr key={item.interface} className="border-t border-border/70 hover:bg-white/[.025]">
                  <Cell><span className="font-mono font-semibold text-cyan-300">{item.interface}</span></Cell>
                  <Cell mono>{formatRate(item.rx_bytes_per_sec)}</Cell>
                  <Cell mono>{formatRate(item.tx_bytes_per_sec)}</Cell>
                  <Cell mono>{item.rx_packets_per_sec.toFixed(1)} / {item.tx_packets_per_sec.toFixed(1)}</Cell>
                  <Cell mono>{item.rx_errors + item.tx_errors}</Cell>
                  <Cell mono>{item.rx_dropped + item.tx_dropped}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        </DataPanel>

        <DataPanel icon={<ServerCog />} title="Systemd services" count={telemetry?.services.length ?? 0} empty="No services configured for observation">
          <table className="w-full min-w-[620px] text-left text-sm">
            <thead><TableHead labels={['Unit', 'State', 'PID', 'Restarts', 'Active for']} /></thead>
            <tbody>
              {telemetry?.services.map(service => (
                <tr key={service.name} className="border-t border-border/70 hover:bg-white/[.025]">
                  <Cell><span className="font-medium text-foreground">{service.name}</span></Cell>
                  <Cell><Status state={service.active_state} detail={service.sub_state} /></Cell>
                  <Cell mono>{service.main_pid || '—'}</Cell>
                  <Cell mono>{service.restart_count}</Cell>
                  <Cell mono>{formatDuration(service.active_for_seconds)}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        </DataPanel>

        <DataPanel icon={<Box />} title="Docker containers" count={telemetry?.containers.length ?? 0} empty="Docker telemetry is disabled or no containers are present">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead><TableHead labels={['Container', 'State', 'CPU', 'Memory', 'Restarts', 'Uptime']} /></thead>
            <tbody>
              {telemetry?.containers.map(container => (
                <tr key={container.id} className="border-t border-border/70 hover:bg-white/[.025]">
                  <Cell><div className="font-medium text-foreground">{container.name}</div><div className="max-w-[240px] truncate text-xs text-muted-foreground">{container.image}</div></Cell>
                  <Cell><Status state={container.state === 'running' && container.health !== 'unhealthy' ? 'active' : container.state} detail={container.health} /></Cell>
                  <Cell mono>{container.cpu_percent.toFixed(1)}%</Cell>
                  <Cell mono>{formatBytes(container.memory_bytes)} <span className="text-muted-foreground">({container.memory_percent.toFixed(1)}%)</span></Cell>
                  <Cell mono>{container.restart_count}</Cell>
                  <Cell mono>{formatDuration(container.run_time_seconds)}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        </DataPanel>
      </div>
    </section>
  );
}

function SignalCard({ icon, label, value, tone = 'normal', children }: { icon: ReactNode; label: string; value: string; tone?: 'normal' | 'danger'; children: ReactNode }) {
  return (
    <div className={`surface-panel rounded-[20px] p-5 ${tone === 'danger' ? 'border-rose-400/30' : ''}`}>
      <div className="flex items-start justify-between gap-4">
        <div><p className="eyebrow">{label}</p><p className={`mt-2 text-2xl font-semibold tracking-tight ${tone === 'danger' ? 'text-rose-300' : 'text-white'}`}>{value}</p></div>
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-cyan-400/8 text-cyan-300 ring-1 ring-cyan-300/10 [&>svg]:h-5 [&>svg]:w-5">{icon}</div>
      </div>
      <div className="mt-3 text-xs leading-5 text-muted-foreground">{children}</div>
    </div>
  );
}

function DataPanel({ icon, title, count, empty, children }: { icon: ReactNode; title: string; count: number; empty: string; children: ReactNode }) {
  return (
    <div className="surface-panel overflow-hidden rounded-[20px]">
      <div className="flex items-center justify-between border-b border-border p-5">
        <div className="flex items-center gap-2.5 text-foreground"><span className="text-accent [&>svg]:h-5 [&>svg]:w-5">{icon}</span><h3 className="font-semibold">{title}</h3></div>
        <span className="rounded-full bg-white/5 px-2.5 py-1 font-mono text-xs text-muted-foreground">{count}</span>
      </div>
      {count > 0 ? <div className="max-h-[360px] overflow-auto">{children}</div> : <div className="p-10 text-center text-sm text-muted-foreground">{empty}</div>}
    </div>
  );
}

function TableHead({ labels }: { labels: string[] }) {
  return <tr className="bg-[#0b111b]/95 text-[10px] uppercase tracking-[.14em] text-muted-foreground">{labels.map(label => <th key={label} className="whitespace-nowrap px-4 py-3 font-medium">{label}</th>)}</tr>;
}

function Cell({ children, mono = false }: { children: ReactNode; mono?: boolean }) {
  return <td className={`whitespace-nowrap px-4 py-3 ${mono ? 'font-mono text-xs text-slate-300' : ''}`}>{children}</td>;
}

function Status({ state, detail }: { state: string; detail: string }) {
  const good = state === 'active' || state === 'running';
  return <span className={`inline-flex items-center gap-1.5 text-xs ${good ? 'text-emerald-300' : 'text-rose-300'}`}><span className="h-1.5 w-1.5 rounded-full bg-current" />{state}{detail && detail !== state ? ` · ${detail}` : ''}</span>;
}

function Usage({ value }: { value: number }) {
  const color = value >= 90 ? 'text-rose-300' : value >= 80 ? 'text-amber-300' : 'text-emerald-300';
  return <span className={`font-mono text-xs ${color}`}>{value.toFixed(1)}%</span>;
}

function formatDuration(totalSeconds: number) {
  if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) return '—';
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

function formatRate(value: number) {
  return `${formatBytes(Math.max(0, value))}/s`;
}

function healthLabel(telemetry: HostTelemetrySnapshot | null, oomDelta: number) {
  if (oomDelta > 0) return 'Attention';
  if (telemetry?.services.some(service => service.active_state !== 'active')) return 'Attention';
  if (telemetry?.containers.some(container => container.state !== 'running' || container.health === 'unhealthy')) return 'Attention';
  return telemetry ? 'Nominal' : 'Waiting';
}

'use client';

import { useMemo, useState, type ReactNode } from 'react';
import { Activity, Clock3, Cpu, MemoryStick, Search, TerminalSquare, UserRound } from 'lucide-react';
import { formatBytes } from '@/lib/metrics-utils';
import type { ProcessSnapshot } from '@/lib/websocket';

interface ProcessWatchProps {
  processes: ProcessSnapshot[];
  updatedAt: string | null;
}

type SortKey = 'cpu_percent' | 'memory_bytes' | 'virtual_memory_bytes' | 'pid' | 'run_time_seconds';

export default function ProcessWatch({ processes, updatedAt }: ProcessWatchProps) {
  const [query, setQuery] = useState('');
  const [sortKey, setSortKey] = useState<SortKey>('cpu_percent');

  const visibleProcesses = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    return processes
      .filter((process) =>
        !normalized ||
        process.command.toLowerCase().includes(normalized) ||
        process.user.toLowerCase().includes(normalized) ||
        String(process.pid).includes(normalized),
      )
      .sort((left, right) => Number(right[sortKey]) - Number(left[sortKey]));
  }, [processes, query, sortKey]);

  const running = processes.filter((process) => process.state === 'run').length;
  const totalMemory = processes.reduce((sum, process) => sum + process.memory_bytes, 0);

  return (
    <section className="surface-panel overflow-hidden rounded-[20px]">
      <div className="flex flex-col gap-4 border-b border-border p-5 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <TerminalSquare className="h-5 w-5 text-accent" />
            <h3 className="font-semibold text-foreground">Live processes</h3>
            <span className="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-2 py-0.5 text-[11px] font-medium text-emerald-300">
              LIVE
            </span>
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {processes.length} processes observed · {running} running · {formatBytes(totalMemory)} resident
            {updatedAt ? ` · updated ${new Date(updatedAt).toLocaleTimeString()}` : ''}
          </p>
        </div>

        <div className="flex flex-col gap-2 sm:flex-row">
          <label className="relative min-w-[220px]">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Filter PID, user, command"
              className="h-10 w-full rounded-xl border border-border bg-background/50 pl-9 pr-3 text-sm text-foreground outline-none transition focus:border-accent/60"
            />
          </label>
          <select
            value={sortKey}
            onChange={(event) => setSortKey(event.target.value as SortKey)}
            className="h-10 rounded-xl border border-border bg-background/50 px-3 text-sm text-foreground outline-none focus:border-accent/60"
            aria-label="Sort processes"
          >
            <option value="cpu_percent">CPU usage</option>
            <option value="memory_bytes">Memory usage</option>
            <option value="virtual_memory_bytes">Virtual memory</option>
            <option value="run_time_seconds">Runtime</option>
            <option value="pid">PID</option>
          </select>
        </div>
      </div>

      {processes.length === 0 ? (
        <div className="p-10 text-center">
          <Activity className="mx-auto mb-3 h-9 w-9 text-muted-foreground" />
          <p className="font-medium text-foreground">Waiting for a process snapshot</p>
          <p className="mt-1 text-sm text-muted-foreground">
            The agent publishes the first table on its next collection interval.
          </p>
        </div>
      ) : (
        <div className="max-h-[620px] overflow-auto">
          <table className="w-full min-w-[1120px] border-collapse text-left text-sm">
            <thead className="sticky top-0 z-10 bg-[#0b111b]/95 text-[11px] uppercase tracking-[0.14em] text-muted-foreground backdrop-blur-xl">
              <tr>
                <Header icon={<TerminalSquare className="h-3.5 w-3.5" />} label="Command" />
                <Header label="PID" />
                <Header label="PPID" />
                <Header icon={<UserRound className="h-3.5 w-3.5" />} label="User" />
                <Header label="State" />
                <Header icon={<Cpu className="h-3.5 w-3.5" />} label="CPU" align="right" />
                <Header icon={<MemoryStick className="h-3.5 w-3.5" />} label="Memory" align="right" />
                <Header label="Virtual" align="right" />
                <Header label="Disk I/O" align="right" />
                <Header icon={<Clock3 className="h-3.5 w-3.5" />} label="Uptime" align="right" />
              </tr>
            </thead>
            <tbody className="font-mono text-[13px]">
              {visibleProcesses.map((process) => (
                <tr key={process.pid} className="border-t border-border/70 transition-colors hover:bg-white/[0.035]">
                  <td className="px-5 py-3 font-semibold text-foreground">{process.command}</td>
                  <td className="px-4 py-3 text-cyan-300">{process.pid}</td>
                  <td className="px-4 py-3 text-muted-foreground">{process.parent_pid ?? '—'}</td>
                  <td className="px-4 py-3 text-muted-foreground">{process.user}</td>
                  <td className="px-4 py-3">
                    <span className={`inline-flex items-center gap-1.5 ${stateColor(process.state)}`}>
                      <span className="h-1.5 w-1.5 rounded-full bg-current" />
                      {process.state}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-right text-amber-300">{process.cpu_percent.toFixed(1)}%</td>
                  <td className="px-4 py-3 text-right text-violet-300">{formatBytes(process.memory_bytes)}</td>
                  <td className="px-4 py-3 text-right text-sky-300">{formatBytes(process.virtual_memory_bytes)}</td>
                  <td className="px-4 py-3 text-right text-muted-foreground">
                    {formatBytes(process.disk_read_bytes)} / {formatBytes(process.disk_written_bytes)}
                  </td>
                  <td className="px-5 py-3 text-right text-muted-foreground">{formatDuration(process.run_time_seconds)}</td>
                </tr>
              ))}
            </tbody>
          </table>
          {visibleProcesses.length === 0 && (
            <p className="p-8 text-center text-sm text-muted-foreground">No process matches “{query}”.</p>
          )}
        </div>
      )}
    </section>
  );
}

function Header({ icon, label, align = 'left' }: { icon?: ReactNode; label: string; align?: 'left' | 'right' }) {
  return (
    <th className={`px-4 py-3 font-medium first:pl-5 last:pr-5 ${align === 'right' ? 'text-right' : ''}`}>
      <span className={`inline-flex items-center gap-1.5 ${align === 'right' ? 'justify-end' : ''}`}>{icon}{label}</span>
    </th>
  );
}

function stateColor(state: string) {
  if (state === 'run') return 'text-emerald-300';
  if (state === 'zombie' || state === 'dead') return 'text-red-300';
  if (state === 'stop' || state === 'tracing') return 'text-amber-300';
  return 'text-slate-400';
}

function formatDuration(totalSeconds: number) {
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = Math.floor(totalSeconds % 60);
  if (days > 0) return `${days}d ${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
  return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

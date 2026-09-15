import { Cpu, MemoryStick, Radio, ServerCog } from 'lucide-react';
import type { ReactNode } from 'react';
import { LatestMetrics } from '@/types';
import { formatBytes } from '@/lib/metrics-utils';

interface ProcessWatchProps {
  metrics: LatestMetrics | null;
}

interface ProcessSnapshot {
  key: string;
  running: number;
  instances: number;
  cpu_usage: number;
  memory_bytes: number;
}

const PROCESS_METRIC = /^process_(.+)_(running|instances|cpu_usage|memory_bytes)$/;

export default function ProcessWatch({ metrics }: ProcessWatchProps) {
  const snapshots = new Map<string, ProcessSnapshot>();

  for (const metric of metrics?.metrics ?? []) {
    const match = metric.name.match(PROCESS_METRIC);
    if (!match) continue;

    const [, key, field] = match;
    const snapshot = snapshots.get(key) ?? {
      key,
      running: 0,
      instances: 0,
      cpu_usage: 0,
      memory_bytes: 0,
    };
    switch (field) {
      case 'running':
        snapshot.running = metric.value;
        break;
      case 'instances':
        snapshot.instances = metric.value;
        break;
      case 'cpu_usage':
        snapshot.cpu_usage = metric.value;
        break;
      case 'memory_bytes':
        snapshot.memory_bytes = metric.value;
        break;
    }
    snapshots.set(key, snapshot);
  }

  const processes = Array.from(snapshots.values()).sort((a, b) =>
    a.key.localeCompare(b.key),
  );

  return (
    <section className="surface-panel overflow-hidden rounded-[20px]">
      {processes.length === 0 ? (
        <div className="p-8 text-center">
          <ServerCog className="w-9 h-9 mx-auto mb-3 text-muted-foreground" />
          <p className="font-medium text-foreground">No watched processes configured</p>
          <p className="text-sm text-muted-foreground mt-1">
            Enable <code className="text-accent">[process_watch]</code> in the agent configuration.
          </p>
        </div>
      ) : (
        <div className="divide-y divide-border">
          {processes.map((process) => {
            const running = process.running > 0;
            return (
              <div
                key={process.key}
                className="grid grid-cols-2 lg:grid-cols-[minmax(180px,1fr)_repeat(3,minmax(120px,0.6fr))] gap-4 p-5 items-center"
              >
                <div className="col-span-2 lg:col-span-1 flex items-center gap-3">
                  <span className={`w-2.5 h-2.5 rounded-full ${running ? 'bg-emerald-400' : 'bg-red-400'}`} />
                  <div>
                    <p className="font-semibold text-foreground">{process.key.replaceAll('_', ' ')}</p>
                    <p className={`text-xs ${running ? 'text-emerald-400' : 'text-red-400'}`}>
                      {running ? 'Running' : 'Stopped'}
                    </p>
                  </div>
                </div>
                <ProcessValue icon={<Radio className="w-4 h-4" />} label="Instances" value={String(process.instances)} />
                <ProcessValue icon={<Cpu className="w-4 h-4" />} label="CPU" value={`${process.cpu_usage.toFixed(1)}%`} />
                <ProcessValue icon={<MemoryStick className="w-4 h-4" />} label="Memory" value={formatBytes(process.memory_bytes)} />
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

function ProcessValue({ icon, label, value }: { icon: ReactNode; label: string; value: string }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="text-muted-foreground">{icon}</span>
      <div>
        <p className="text-xs text-muted-foreground">{label}</p>
        <p className="font-medium text-foreground">{value}</p>
      </div>
    </div>
  );
}

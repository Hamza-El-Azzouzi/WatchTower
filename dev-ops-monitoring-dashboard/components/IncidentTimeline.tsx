'use client';

import { useEffect, useState } from 'react';
import { Activity, BellRing, CheckCircle2, CircleAlert, FileWarning, Gauge, UserCheck } from 'lucide-react';
import { getIncidentTimeline, type IncidentEvent } from '@/lib/alerts-api';

export default function IncidentTimeline({ agentId }: { agentId: string }) {
  const [events, setEvents] = useState<IncidentEvent[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const result = await getIncidentTimeline(agentId);
        if (active) setEvents(result);
      } catch (error) {
        console.error('Failed to load incident timeline:', error);
      } finally {
        if (active) setLoading(false);
      }
    };
    load();
    const timer = window.setInterval(load, 10_000);
    return () => { active = false; window.clearInterval(timer); };
  }, [agentId]);

  return (
    <section className="surface-panel overflow-hidden rounded-[24px] border border-border/80">
      <div className="border-b border-border/70 p-6">
        <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-accent">
          <Activity className="h-4 w-4" /> Correlated operations
        </div>
        <h2 className="mt-2 text-xl font-semibold">Incident timeline</h2>
        <p className="mt-1 text-sm text-muted-foreground">Alert transitions, recoveries, acknowledgements, process spikes, and error logs in one sequence.</p>
      </div>
      <div className="max-h-[560px] overflow-y-auto p-6">
        {loading && <p className="text-sm text-muted-foreground">Loading incident activity…</p>}
        {!loading && events.length === 0 && (
          <div className="rounded-2xl border border-dashed border-border p-10 text-center text-sm text-muted-foreground">
            No incident activity has been recorded for this agent.
          </div>
        )}
        <div className="space-y-1">
          {events.map((event, index) => {
            const visual = eventVisual(event.event_type, event.severity);
            return (
              <article key={event.event_id} className="relative flex gap-4 pb-6">
                {index < events.length - 1 && <span className="absolute left-[17px] top-9 h-[calc(100%-20px)] w-px bg-border" />}
                <span className={`relative z-10 flex h-9 w-9 shrink-0 items-center justify-center rounded-full border ${visual.surface}`}>
                  <visual.icon className="h-4 w-4" />
                </span>
                <div className="min-w-0 flex-1 rounded-2xl border border-border/70 bg-background/30 p-4">
                  <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                    <h3 className="font-medium text-foreground">{event.title}</h3>
                    <time className="shrink-0 text-xs tabular-nums text-muted-foreground">{new Date(event.occurred_at).toLocaleString()}</time>
                  </div>
                  <p className="mt-2 break-words text-sm text-muted-foreground">{event.description}</p>
                  <span className={`mt-3 inline-flex rounded-full border px-2 py-0.5 text-[11px] font-medium uppercase tracking-wide ${visual.badge}`}>
                    {event.event_type.replace('_', ' ')}
                  </span>
                </div>
              </article>
            );
          })}
        </div>
      </div>
    </section>
  );
}

function eventVisual(type: IncidentEvent['event_type'], severity: IncidentEvent['severity']) {
  const icon = type === 'recovery' ? CheckCircle2
    : type === 'acknowledged' ? UserCheck
    : type === 'process_spike' ? Gauge
    : type === 'log_error' ? FileWarning
    : type === 'firing' ? BellRing : CircleAlert;
  if (type === 'recovery' || type === 'acknowledged') {
    return { icon, surface: 'border-emerald-400/30 bg-emerald-400/10 text-emerald-300', badge: 'border-emerald-400/20 bg-emerald-400/10 text-emerald-300' };
  }
  if (severity === 'critical') {
    return { icon, surface: 'border-red-400/30 bg-red-400/10 text-red-300', badge: 'border-red-400/20 bg-red-400/10 text-red-300' };
  }
  return { icon, surface: 'border-amber-400/30 bg-amber-400/10 text-amber-300', badge: 'border-amber-400/20 bg-amber-400/10 text-amber-300' };
}

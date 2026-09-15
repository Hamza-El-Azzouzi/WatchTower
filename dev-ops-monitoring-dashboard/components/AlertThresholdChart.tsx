'use client';

import { useEffect, useMemo, useState, type ReactNode } from 'react';
import {
  Area, CartesianGrid, ComposedChart, Line, ReferenceArea, ReferenceLine,
  ResponsiveContainer, Tooltip, XAxis, YAxis,
} from 'recharts';
import {
  Activity, AlertTriangle, CheckCircle2, Clock3, Gauge,
  TrendingDown, TrendingUp, XCircle,
} from 'lucide-react';
import { getHistoricalMetrics } from '@/lib/api';
import { formatChartTime } from '@/lib/metrics-utils';
import type { DataPoint, Metric } from '@/types';

interface AlertThresholdChartProps {
  agentId: string;
  metric: 'cpu_usage' | 'memory_usage' | 'disk_usage';
  title: string;
  warningThreshold?: number;
  criticalThreshold?: number;
  limit?: number;
  liveMetric?: Metric;
}

interface ChartPoint {
  timestamp: string;
  rawTimestamp: string;
  value: number;
}

type AlertStatus = 'ok' | 'warning' | 'critical';
const MAX_LIVE_POINTS = 120;

export default function AlertThresholdChart({
  agentId,
  metric,
  title,
  warningThreshold,
  criticalThreshold,
  limit = 120,
  liveMetric,
}: AlertThresholdChartProps) {
  const [data, setData] = useState<ChartPoint[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const monitoringConfigured = warningThreshold !== undefined || criticalThreshold !== undefined;

  useEffect(() => {
    let active = true;
    const loadHistory = async () => {
      try {
        setLoading(true);
        const result = await getHistoricalMetrics(agentId, metric, limit);
        if (!active) return;
        const history = result.data_points.map((point: DataPoint) => ({
          timestamp: formatChartTime(point.timestamp),
          rawTimestamp: point.timestamp,
          value: Number(point.value.toFixed(2)),
        }));
        setData(previous => mergePoints(history, previous));
        setLoadError(false);
      } catch (error) {
        console.error(`Error fetching ${metric}:`, error);
        if (active) setLoadError(true);
      } finally {
        if (active) setLoading(false);
      }
    };
    loadHistory();
    return () => { active = false; };
  }, [agentId, metric, limit]);

  useEffect(() => {
    if (!liveMetric) return;
    const point: ChartPoint = {
      timestamp: formatChartTime(liveMetric.timestamp),
      rawTimestamp: liveMetric.timestamp,
      value: Number(liveMetric.value.toFixed(2)),
    };
    setData(previous => {
      if (previous.at(-1)?.rawTimestamp === point.rawTimestamp) return previous;
      return mergePoints(previous, [point]);
    });
  }, [liveMetric]);

  const summary = useMemo(() => {
    const values = data.map(point => point.value);
    const current = values.at(-1) ?? 0;
    const average = values.length
      ? values.reduce((total, value) => total + value, 0) / values.length
      : 0;
    const peak = values.length ? Math.max(...values) : 0;
    const previous = values.length > 1 ? values.at(-2) ?? current : current;
    const nextThreshold = [warningThreshold, criticalThreshold]
      .filter((value): value is number => value !== undefined && value > current)
      .sort((left, right) => left - right)[0];
    const status: AlertStatus = criticalThreshold !== undefined && current >= criticalThreshold
      ? 'critical'
      : warningThreshold !== undefined && current >= warningThreshold ? 'warning' : 'ok';
    return {
      current,
      average,
      peak,
      trend: current - previous,
      status,
      headroom: nextThreshold === undefined ? null : nextThreshold - current,
      latestTimestamp: data.at(-1)?.rawTimestamp,
    };
  }, [criticalThreshold, data, warningThreshold]);

  const statusStyle = {
    ok: { label: 'Normal', icon: <CheckCircle2 className="h-5 w-5" />, text: 'text-emerald-300', surface: 'border-emerald-400/20 bg-emerald-400/10' },
    warning: { label: 'Warning', icon: <AlertTriangle className="h-5 w-5" />, text: 'text-amber-300', surface: 'border-amber-400/20 bg-amber-400/10' },
    critical: { label: 'Critical', icon: <XCircle className="h-5 w-5" />, text: 'text-red-300', surface: 'border-red-400/20 bg-red-400/10' },
  }[summary.status];

  if (loading && data.length === 0) {
    return (
      <div className="surface-panel min-h-[590px] animate-pulse rounded-[24px] p-7">
        <div className="mb-6 h-8 w-48 rounded-lg bg-white/5" />
        <div className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
          {[0, 1, 2, 3].map(item => <div key={item} className="h-24 rounded-2xl bg-white/5" />)}
        </div>
        <div className="h-[380px] rounded-2xl bg-white/5" />
      </div>
    );
  }

  return (
    <article className="surface-panel overflow-hidden rounded-[24px] border border-border/80">
      <div className="border-b border-border/70 p-6 lg:p-7">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
              <Activity className="h-4 w-4 text-accent" /> Live resource telemetry
            </div>
            <h3 className="text-xl font-semibold text-foreground">{title}</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              {monitoringConfigured
                ? `${warningThreshold !== undefined ? `Warning at ${warningThreshold}%` : ''}${warningThreshold !== undefined && criticalThreshold !== undefined ? ' · ' : ''}${criticalThreshold !== undefined ? `Critical at ${criticalThreshold}%` : ''}`
                : 'No enabled alert rule is configured for this metric'}
            </p>
          </div>
          <div className={`flex min-w-[154px] items-center justify-between gap-4 rounded-2xl border px-4 py-3 ${statusStyle.surface}`}>
            <span className={statusStyle.text}>{statusStyle.icon}</span>
            <div className="text-right">
              <p className={`text-3xl font-semibold tabular-nums ${statusStyle.text}`}>{summary.current.toFixed(1)}%</p>
              <p className={`text-xs font-medium ${statusStyle.text}`}>{monitoringConfigured ? statusStyle.label : 'Observed'}</p>
            </div>
          </div>
        </div>

        <div className="mt-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
          <SummaryCard icon={<Gauge />} label="Average" value={`${summary.average.toFixed(1)}%`} />
          <SummaryCard icon={<TrendingUp />} label="Peak" value={`${summary.peak.toFixed(1)}%`} />
          <SummaryCard
            icon={summary.trend >= 0 ? <TrendingUp /> : <TrendingDown />}
            label="Last movement"
            value={`${summary.trend >= 0 ? '+' : ''}${summary.trend.toFixed(1)} pts`}
          />
          <SummaryCard
            icon={<AlertTriangle />}
            label="Next threshold"
            value={!monitoringConfigured ? 'No rule' : summary.headroom !== null ? `${summary.headroom.toFixed(1)} pts` : 'Threshold crossed'}
            danger={summary.status !== 'ok'}
          />
        </div>
      </div>

      <div className="p-4 sm:p-6 lg:p-7">
        {loadError && data.length === 0 ? (
          <div className="flex h-[400px] items-center justify-center rounded-2xl border border-dashed border-border text-sm text-muted-foreground">
            Historical data is unavailable. Waiting for the next live sample…
          </div>
        ) : (
          <div className="h-[400px] w-full lg:h-[460px]">
            <ResponsiveContainer width="100%" height="100%">
              <ComposedChart data={data} margin={{ top: 18, right: 36, bottom: 8, left: 0 }}>
                <defs>
                  <linearGradient id={`gradient-${metric}`} x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#38bdf8" stopOpacity={0.42} />
                    <stop offset="80%" stopColor="#38bdf8" stopOpacity={0.04} />
                  </linearGradient>
                </defs>
                <CartesianGrid vertical={false} strokeDasharray="4 6" stroke="rgba(148,163,184,0.12)" />
                <XAxis dataKey="timestamp" minTickGap={44} tickLine={false} axisLine={false} stroke="#94a3b8" tick={{ fontSize: 11 }} />
                <YAxis domain={[0, 100]} width={44} tickLine={false} axisLine={false} stroke="#94a3b8" tick={{ fontSize: 11 }} tickFormatter={value => `${value}%`} />
                {criticalThreshold !== undefined && <ReferenceArea y1={criticalThreshold} y2={100} fill="#ef4444" fillOpacity={0.09} />}
                {warningThreshold !== undefined && <ReferenceArea y1={warningThreshold} y2={criticalThreshold ?? 100} fill="#f59e0b" fillOpacity={0.08} />}
                {warningThreshold !== undefined && <ReferenceLine y={warningThreshold} stroke="#f59e0b" strokeDasharray="7 6" />}
                {criticalThreshold !== undefined && <ReferenceLine y={criticalThreshold} stroke="#ef4444" strokeDasharray="7 6" />}
                <Tooltip
                  cursor={{ stroke: 'rgba(148,163,184,.35)', strokeDasharray: '4 4' }}
                  contentStyle={{ backgroundColor: 'rgba(7,11,18,.96)', border: '1px solid rgba(148,163,184,.22)', borderRadius: 14, boxShadow: '0 18px 45px rgba(0,0,0,.35)' }}
                  labelStyle={{ color: '#94a3b8', marginBottom: 4 }}
                  formatter={(value: number) => [`${Number(value).toFixed(2)}%`, title]}
                />
                <Area type="monotone" dataKey="value" fill={`url(#gradient-${metric})`} stroke="none" isAnimationActive={false} />
                <Line type="monotone" dataKey="value" stroke="#38bdf8" strokeWidth={2.5} dot={false} activeDot={{ r: 5, fill: '#38bdf8', stroke: '#07101b', strokeWidth: 3 }} isAnimationActive={false} />
              </ComposedChart>
            </ResponsiveContainer>
          </div>
        )}

        <div className="mt-4 flex flex-col gap-3 border-t border-border/70 pt-4 text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
            {!monitoringConfigured && <span>No configured guardrail — create an alert rule to enable threshold zones.</span>}
            {warningThreshold !== undefined && <LegendDot color="bg-amber-400" label={`Warning ≥ ${warningThreshold}%`} />}
            {criticalThreshold !== undefined && <LegendDot color="bg-red-400" label={`Critical ≥ ${criticalThreshold}%`} />}
          </div>
          <span className="inline-flex items-center gap-1.5 tabular-nums">
            <Clock3 className="h-3.5 w-3.5" />
            {summary.latestTimestamp ? `Latest ${new Date(summary.latestTimestamp).toLocaleTimeString()}` : 'Waiting for telemetry'}
          </span>
        </div>
      </div>
    </article>
  );
}

function SummaryCard({ icon, label, value, danger = false }: { icon: ReactNode; label: string; value: string; danger?: boolean }) {
  return (
    <div className="rounded-2xl border border-border/70 bg-background/35 px-4 py-3.5">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="[&>svg]:h-3.5 [&>svg]:w-3.5">{icon}</span>{label}
      </div>
      <p className={`mt-2 text-sm font-semibold tabular-nums ${danger ? 'text-amber-300' : 'text-foreground'}`}>{value}</p>
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return <span className="inline-flex items-center gap-2"><span className={`h-2 w-2 rounded-full ${color}`} />{label}</span>;
}

function mergePoints(...groups: ChartPoint[][]) {
  const unique = new Map<string, ChartPoint>();
  groups.flat().forEach(point => unique.set(point.rawTimestamp, point));
  return Array.from(unique.values())
    .sort((left, right) => Date.parse(left.rawTimestamp) - Date.parse(right.rawTimestamp))
    .slice(-MAX_LIVE_POINTS);
}

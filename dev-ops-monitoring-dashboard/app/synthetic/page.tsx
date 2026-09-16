'use client';
import { useCallback, useEffect, useRef, useState, type FormEvent } from 'react';
import { Globe, Plus, Clock3, ShieldCheck, Activity, Pause, Play, Trash2, X, AlertCircle } from 'lucide-react';
import { ResponsiveContainer, AreaChart, Area, CartesianGrid, XAxis, YAxis, Tooltip } from 'recharts';
import { listSyntheticChecks, createSyntheticCheck, setSyntheticEnabled, deleteSyntheticCheck, getSyntheticHistory, type CheckSpec, type CheckKind, type SyntheticCheck, type CheckHistory } from '@/lib/synthetic-api';
import { getNotificationChannels, type NotificationChannel } from '@/lib/alerts-api';

const input = 'w-full rounded-lg border border-slate-700 bg-slate-950/60 px-3 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-primary';
const initial: CheckSpec = { name: '', kind: 'http', target: '', interval_seconds: 60, timeout_seconds: 5, failure_threshold: 3, expected_status: 200, dns_record_type: 'A', tls_expiry_days: 14, channels: [] };
function status(check: SyntheticCheck) {
  if (!check.enabled) return 'Paused';
  if (!check.last_result) return 'Pending';
  if (Date.now() - new Date(check.last_result.checked_at).getTime() > (check.spec.interval_seconds + check.spec.timeout_seconds + 30) * 1000) return 'Stale';
  if (check.last_result.success) return 'Healthy';
  return check.active_alert_id ? 'Failing' : 'Confirming';
}
function statusStyle(value: string) { return value === 'Healthy' ? 'text-emerald-400 bg-emerald-500/10' : value === 'Failing' || value === 'Stale' ? 'text-rose-400 bg-rose-500/10' : 'text-amber-300 bg-amber-500/10'; }

export default function SyntheticPage() {
  const [checks, setChecks] = useState<SyntheticCheck[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selectedRef = useRef<string | null>(null);
  useEffect(() => { selectedRef.current = selectedId; }, [selectedId]);
  const [history, setHistory] = useState<CheckHistory[]>([]);
  const [channels, setChannels] = useState<NotificationChannel[]>([]);
  const [channelsError, setChannelsError] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [draft, setDraft] = useState<CheckSpec>(initial);
  const [busy, setBusy] = useState(false);
  const [updatedAt, setUpdatedAt] = useState<Date | null>(null);
  const refresh = useCallback(async () => {
    try {
      const values = await listSyntheticChecks(); setChecks(values); setUpdatedAt(new Date());
      setSelectedId(current => values.some(check => check.id === current) ? current : values[0]?.id ?? null);
      if (selectedId) {
        const entries = await getSyntheticHistory(selectedId);
        if (selectedRef.current === selectedId) setHistory(entries);
      }
      setError(null);
    } catch (failure) { setError(failure instanceof Error ? failure.message : 'Unable to load checks'); }
    finally { setLoading(false); }
  }, [selectedId]);
  useEffect(() => {
    let stopped = false; let timer: ReturnType<typeof setTimeout>;
    async function tick() { if (document.visibilityState === 'visible') await refresh(); if (!stopped) timer = setTimeout(tick, 5000); }
    tick();
    return () => { stopped = true; clearTimeout(timer); };
  }, [refresh]);
  useEffect(() => { getNotificationChannels().then(setChannels).catch(() => setChannelsError(true)); }, []);
  const selected = checks.find(check => check.id === selectedId);
  const healthy = checks.filter(check => status(check) === 'Healthy').length;
  const failures = checks.filter(check => check.enabled && (status(check) === 'Failing' || status(check) === 'Confirming' || status(check) === 'Stale')).length;
  const availability = history.length ? (history.filter(entry => entry.result.success).length / history.length * 100).toFixed(1) : null;
  const chart = [...history].reverse().map(entry => ({ time: new Date(entry.result.checked_at).toLocaleTimeString(), latency: Math.round(entry.result.response_time_ms), outcome: entry.result.success ? 'Healthy' : 'Failed' }));
  async function submit(event: FormEvent) {
    event.preventDefault(); setBusy(true); setError(null);
    try {
      const { id } = await createSyntheticCheck({ ...draft, target: draft.target.trim(), name: draft.name.trim(), expected_content: draft.expected_content || undefined, expected_dns_value: draft.expected_dns_value || undefined });
      setSelectedId(id); setShowForm(false); setDraft(initial); await refresh();
    } catch (failure) { setError(failure instanceof Error ? failure.message : 'Unable to create check'); }
    finally { setBusy(false); }
  }
  async function change(check: SyntheticCheck, remove = false) {
    if (remove && !confirm(`Delete ${check.spec.name}? Monitoring stops; history is retained for 30 days.`)) return;
    setBusy(true);
    try { if (remove) await deleteSyntheticCheck(check.id); else await setSyntheticEnabled(check.id, !check.enabled); await refresh(); }
    catch (failure) { setError(failure instanceof Error ? failure.message : 'Check update failed'); }
    finally { setBusy(false); }
  }
  const field = (label: string, id: string, control: React.ReactNode) => <div><label htmlFor={id} className="block text-sm font-medium mb-2">{label}</label>{control}</div>;
  return <div className="p-5 md:p-8 max-w-[1600px] mx-auto space-y-6">
    <header className="flex flex-wrap items-start justify-between gap-4">
      <div><div className="flex items-center gap-3"><Globe className="text-primary h-7 w-7" /><h1 className="text-3xl font-semibold tracking-tight">Synthetic monitoring</h1></div><p className="text-muted-foreground mt-2">Verify application reachability, response integrity, and certificate health.</p></div>
      <button onClick={() => setShowForm(!showForm)} className="flex items-center gap-2 rounded-lg bg-primary px-4 py-2.5 text-primary-foreground"><Plus size={16} />Create check</button>
    </header>
    <div className="flex flex-wrap gap-4 text-xs text-muted-foreground"><span>Probe location: Oracle server</span><span>Public destinations only</span><span>History: 30 days / 10,000 results per check</span><span>{updatedAt ? `Updated ${updatedAt.toLocaleTimeString()}` : 'Connecting…'}</span></div>
    {error && <div role="alert" className="flex items-center gap-3 rounded-lg border border-rose-500/30 bg-rose-500/10 p-4 text-rose-300"><AlertCircle size={18} /><span>{error}</span></div>}
    <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">{[
      { label: 'Total checks', value: checks.length, icon: Globe }, { label: 'Healthy', value: healthy, icon: ShieldCheck },
      { label: 'Needs attention', value: failures, icon: Activity }, { label: 'Paused', value: checks.filter(check => !check.enabled).length, icon: Pause },
    ].map(card => <div key={card.label} className="surface-panel p-5"><div className="flex justify-between text-muted-foreground text-sm">{card.label}<card.icon size={17} /></div><div className="text-3xl font-semibold mt-3 tabular-nums">{card.value}</div></div>)}</div>
    {showForm && <form onSubmit={submit} className="surface-panel p-5 md:p-7 space-y-6">
      <div className="flex justify-between"><h2 className="font-semibold text-xl">Configure an availability check</h2><button type="button" onClick={() => setShowForm(false)} aria-label="Close check form"><X size={20} /></button></div>
      <div className="grid md:grid-cols-2 xl:grid-cols-3 gap-5">
        {field('Check name', 'check-name', <input id="check-name" required maxLength={128} value={draft.name} onChange={event => setDraft({ ...draft, name: event.target.value })} className={input} placeholder="Production API" />)}
        {field('Check type', 'check-kind', <select id="check-kind" className={input} value={draft.kind} onChange={event => setDraft({ ...draft, kind: event.target.value as CheckKind, target: '', port: event.target.value === 'tcp' ? 443 : undefined })}><option value="http">HTTP / HTTPS</option><option value="tcp">TCP port</option><option value="dns">DNS resolution</option><option value="tls">TLS certificate</option></select>)}
        {field(draft.kind === 'http' ? 'Request URL' : 'Hostname / IPv4 address', 'check-target', <input id="check-target" required maxLength={2048} type={draft.kind === 'http' ? 'url' : 'text'} className={input} value={draft.target} onChange={event => setDraft({ ...draft, target: event.target.value })} placeholder={draft.kind === 'http' ? 'https://api.example.com/health' : 'api.example.com'} />)}
        {(draft.kind === 'tcp' || draft.kind === 'tls') && field('Port', 'check-port', <input id="check-port" type="number" required min={1} max={65535} className={input} value={draft.port ?? 443} onChange={event => setDraft({ ...draft, port: Number(event.target.value) })} />)}
        {field('Check interval (seconds)', 'check-interval', <input id="check-interval" type="number" min={30} max={3600} required className={input} value={draft.interval_seconds} onChange={event => setDraft({ ...draft, interval_seconds: Number(event.target.value) })} />)}
        {field('Timeout (seconds)', 'check-timeout', <input id="check-timeout" type="number" min={1} max={10} required className={input} value={draft.timeout_seconds} onChange={event => setDraft({ ...draft, timeout_seconds: Number(event.target.value) })} />)}
        {field('Consecutive failures before alerting', 'check-failures', <input id="check-failures" type="number" min={1} max={10} required className={input} value={draft.failure_threshold} onChange={event => setDraft({ ...draft, failure_threshold: Number(event.target.value) })} />)}
        {draft.kind === 'http' && <>{field('Expected HTTP status', 'check-status', <input id="check-status" type="number" min={100} max={599} required className={input} value={draft.expected_status} onChange={event => setDraft({ ...draft, expected_status: Number(event.target.value) })} />)}{field('Expected response text (optional)', 'check-content', <input id="check-content" maxLength={4096} className={input} value={draft.expected_content ?? ''} onChange={event => setDraft({ ...draft, expected_content: event.target.value })} placeholder='"status":"healthy"' />)}</>}
        {draft.kind === 'dns' && <>{field('DNS record type', 'check-dns-type', <select id="check-dns-type" className={input} value={draft.dns_record_type} onChange={event => setDraft({ ...draft, dns_record_type: event.target.value as 'A' | 'AAAA' })}><option>A</option><option>AAAA</option></select>)}{field('Expected IP address (optional)', 'check-dns-value', <input id="check-dns-value" className={input} value={draft.expected_dns_value ?? ''} onChange={event => setDraft({ ...draft, expected_dns_value: event.target.value })} />)}</>}
        {draft.kind === 'tls' && field('Alert when certificate expires within (days)', 'check-tls-days', <input id="check-tls-days" type="number" min={0} max={365} required className={input} value={draft.tls_expiry_days} onChange={event => setDraft({ ...draft, tls_expiry_days: Number(event.target.value) })} />)}
      </div>
      <fieldset><legend className="text-sm font-medium mb-3">Notification channels</legend><div className="flex flex-wrap gap-3">{channels.filter(channel => channel.enabled).map(channel => <label key={channel.id} className="flex items-center gap-2 border border-slate-700 rounded-lg px-3 py-2 text-sm"><input type="checkbox" checked={draft.channels.includes(channel.id)} onChange={event => setDraft({ ...draft, channels: event.target.checked ? [...draft.channels, channel.id] : draft.channels.filter(id => id !== channel.id) })} />{channel.name}</label>)}</div><p className={`text-xs mt-3 ${channelsError ? 'text-amber-300' : 'text-muted-foreground'}`}>{channelsError ? 'Channels could not be loaded. Reload this page before configuring notifications.' : channels.length ? 'Selected channels receive firing and recovery notifications.' : 'Add channels on the Alerts page to enable notifications. Incidents still appear without a channel.'}</p></fieldset>
      <div className="flex flex-wrap justify-between gap-4 border-t border-slate-800 pt-5"><p className="text-sm text-muted-foreground max-w-2xl">Alert after {draft.failure_threshold} consecutive failures. One successful check resolves the incident. HTTP redirects are not followed; monitor the final URL. Response bodies are not stored.</p><button disabled={busy} className="rounded-lg bg-primary px-5 py-2.5 text-primary-foreground disabled:opacity-50">{busy ? 'Creating…' : 'Create check'}</button></div>
    </form>}
    {loading ? <div className="surface-panel p-12 text-center text-muted-foreground">Loading checks…</div> : !checks.length ? <div className="surface-panel p-12 text-center"><Globe className="mx-auto text-slate-500 mb-4" size={36} /><h2 className="text-xl font-semibold">Monitor more than resource usage</h2><p className="text-muted-foreground mt-2">Create your first check to catch unreachable services, unexpected responses, and expiring certificates.</p></div> : <div className="grid xl:grid-cols-[340px_1fr] gap-6">
      <div className="space-y-3">{checks.map(check => { const value = status(check); return <button key={check.id} onClick={() => { setSelectedId(check.id); setHistory([]); }} className={`surface-panel w-full text-left p-5 transition-colors ${selectedId === check.id ? 'ring-1 ring-primary' : 'hover:border-slate-600'}`}><div className="flex items-start justify-between gap-2"><span className="font-semibold truncate">{check.spec.name}</span><span className={`text-xs rounded-md px-2 py-1 ${statusStyle(value)}`}>{value}</span></div><p className="text-xs text-muted-foreground mt-2 truncate">{check.spec.target}</p><div className="flex justify-between mt-4 text-xs text-muted-foreground"><span>{check.spec.kind.toUpperCase()} · {check.spec.interval_seconds}s</span><span>{check.last_result ? `${check.last_result.response_time_ms.toFixed(0)} ms` : 'Not checked yet'}</span></div></button>; })}</div>
      {selected && <section className="surface-panel p-5 md:p-7 min-w-0 space-y-6">
        <div className="flex flex-wrap justify-between gap-4"><div><h2 className="text-xl font-semibold">{selected.spec.name}</h2><p className="text-sm text-muted-foreground mt-1 break-all">{selected.spec.target}{selected.spec.port ? `:${selected.spec.port}` : ''}</p></div><div className="flex gap-2"><button disabled={busy} onClick={() => change(selected)} className="rounded-lg border border-slate-700 px-3 py-2 flex items-center gap-2 text-sm disabled:opacity-50">{selected.enabled ? <Pause size={15} /> : <Play size={15} />}{selected.enabled ? 'Pause' : 'Resume'}</button><button disabled={busy} aria-label={`Delete ${selected.spec.name}`} onClick={() => change(selected, true)} className="rounded-lg border border-slate-700 p-2 text-rose-400 disabled:opacity-50"><Trash2 size={16} /></button></div></div>
        {selected.last_result?.error && <p className="rounded-lg border border-rose-500/20 bg-rose-500/5 p-3 text-sm text-rose-300">{selected.last_result.error}</p>}
        {!selected.enabled && selected.active_alert_id && <p className="text-sm text-amber-300">Monitoring is paused. The active incident stays open until a successful check confirms recovery.</p>}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm"><div><p className="text-muted-foreground">Latest response</p><p className="font-semibold mt-1 tabular-nums">{selected.last_result ? `${selected.last_result.response_time_ms.toFixed(0)} ms` : '—'}</p></div><div><p className="text-muted-foreground">Success rate · shown samples</p><p className="font-semibold mt-1">{availability ? `${availability}%` : '—'}</p></div><div><p className="text-muted-foreground">Failures / alert threshold</p><p className="font-semibold mt-1">{selected.consecutive_failures} / {selected.spec.failure_threshold}</p></div><div><p className="text-muted-foreground">Certificate remaining</p><p className="font-semibold mt-1">{selected.last_result?.tls_days_remaining != null ? `${selected.last_result.tls_days_remaining.toFixed(1)} days` : '—'}</p></div></div>
        <div><div className="flex gap-2 items-center mb-4 text-sm font-medium"><Clock3 size={16} />Response-time history <span className="text-xs text-muted-foreground font-normal">latest 100 probes</span></div><div className="h-[320px] w-full">{chart.length ? <ResponsiveContainer width="100%" height="100%"><AreaChart data={chart}><defs><linearGradient id="syntheticLatency" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor="#38bdf8" stopOpacity={0.3} /><stop offset="100%" stopColor="#38bdf8" stopOpacity={0} /></linearGradient></defs><CartesianGrid stroke="#243047" strokeDasharray="3 3" /><XAxis dataKey="time" tick={{ fill: '#94a3b8', fontSize: 11 }} minTickGap={65} /><YAxis tick={{ fill: '#94a3b8', fontSize: 11 }} unit=" ms" width={70} /><Tooltip contentStyle={{ background: '#0f172a', border: '1px solid #334155', borderRadius: 8 }} formatter={(value, _name, item) => [`${value} ms · ${item.payload.outcome}`, 'Response']} /><Area type="linear" dataKey="latency" stroke="#38bdf8" fill="url(#syntheticLatency)" isAnimationActive={false} /></AreaChart></ResponsiveContainer> : <div className="h-full flex items-center justify-center text-sm text-muted-foreground">History appears after the first completed probe.</div>}</div></div>
        <div className="overflow-x-auto"><table className="w-full text-sm"><thead><tr className="text-left text-muted-foreground border-b border-slate-800"><th className="py-3 font-medium">Checked at</th><th className="font-medium">Result</th><th className="font-medium">HTTP status</th><th className="font-medium">Time</th><th className="font-medium">Details</th></tr></thead><tbody>{history.slice(0, 20).map(entry => <tr key={entry.id} className="border-b border-slate-800/60"><td className="py-3 pr-4 whitespace-nowrap">{new Date(entry.result.checked_at).toLocaleString()}</td><td className={entry.result.success ? 'text-emerald-400' : 'text-rose-400'}>{entry.result.success ? 'Healthy' : 'Failed'}</td><td>{entry.result.status_code ?? '—'}</td><td className="whitespace-nowrap pr-4 tabular-nums">{entry.result.response_time_ms.toFixed(0)} ms</td><td className="text-xs text-muted-foreground max-w-xs break-words">{entry.result.error || (entry.result.tls_expires_at ? `Expires ${new Date(entry.result.tls_expires_at).toLocaleDateString()}` : entry.result.content_matched != null ? 'Expected content matched' : entry.result.resolved_addresses.join(', '))}</td></tr>)}</tbody></table></div>
      </section>}
    </div>}
  </div>;
}

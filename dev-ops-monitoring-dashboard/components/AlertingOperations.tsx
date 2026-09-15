'use client';

import { useCallback, useEffect, useState } from 'react';
import { BellRing, CalendarClock, CheckCircle2, FlaskConical, Plus, Send, Trash2, XCircle } from 'lucide-react';
import {
  createAlertSilence, createNotificationChannel, deleteAlertSilence, deleteNotificationChannel,
  getAlertSilences, getNotificationChannels, getNotificationDeliveries,
  setNotificationChannelEnabled, testNotificationChannel,
  type AlertRule, type AlertSilence, type NotificationChannel, type NotificationChannelType,
  type NotificationDelivery,
} from '@/lib/alerts-api';

type Tab = 'channels' | 'maintenance' | 'deliveries';

export default function AlertingOperations({ rules }: { rules: AlertRule[] }) {
  const [tab, setTab] = useState<Tab>('channels');
  const [channels, setChannels] = useState<NotificationChannel[]>([]);
  const [silences, setSilences] = useState<AlertSilence[]>([]);
  const [deliveries, setDeliveries] = useState<NotificationDelivery[]>([]);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const [channelData, silenceData, deliveryData] = await Promise.all([
        getNotificationChannels(), getAlertSilences(), getNotificationDeliveries(),
      ]);
      setChannels(channelData); setSilences(silenceData); setDeliveries(deliveryData); setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : 'Could not load alerting operations');
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);
  useEffect(() => {
    if (tab !== 'deliveries') return;
    const timer = window.setInterval(refresh, 5_000);
    return () => window.clearInterval(timer);
  }, [refresh, tab]);

  return (
    <section className="surface-panel mb-8 overflow-hidden rounded-[24px] border border-border/80">
      <div className="flex flex-col gap-4 border-b border-border/70 p-6 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.16em] text-accent"><BellRing className="h-4 w-4" /> Response automation</div>
          <h2 className="mt-2 text-xl font-semibold">Production alerting</h2>
          <p className="mt-1 text-sm text-muted-foreground">Destinations, maintenance windows, tests, and durable delivery history.</p>
        </div>
        <div className="flex rounded-xl border border-border bg-background/40 p-1">
          {(['channels', 'maintenance', 'deliveries'] as Tab[]).map(item => (
            <button key={item} onClick={() => setTab(item)} className={`rounded-lg px-3 py-2 text-xs font-medium capitalize transition-colors ${tab === item ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground'}`}>{item}</button>
          ))}
        </div>
      </div>
      {error && <div className="m-6 rounded-xl border border-red-400/20 bg-red-400/10 p-3 text-sm text-red-300">{error}</div>}
      <div className="p-6">
        {tab === 'channels' && <ChannelsPanel channels={channels} onChange={refresh} />}
        {tab === 'maintenance' && <MaintenancePanel rules={rules} silences={silences} onChange={refresh} />}
        {tab === 'deliveries' && <DeliveryPanel deliveries={deliveries} />}
      </div>
    </section>
  );
}

function ChannelsPanel({ channels, onChange }: { channels: NotificationChannel[]; onChange: () => Promise<void> }) {
  const [type, setType] = useState<NotificationChannelType>('generic_webhook');
  const [name, setName] = useState('');
  const [destination, setDestination] = useState('');
  const [smtpHost, setSmtpHost] = useState('');
  const [smtpPort, setSmtpPort] = useState(587);
  const [smtpFrom, setSmtpFrom] = useState('');
  const [smtpUsername, setSmtpUsername] = useState('');
  const [smtpPassword, setSmtpPassword] = useState('');
  const [saving, setSaving] = useState(false);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault(); setSaving(true);
    try {
      await createNotificationChannel(type === 'email' ? {
        name, channel_type: type, email_to: destination, smtp_host: smtpHost,
        smtp_port: smtpPort, smtp_from: smtpFrom, smtp_username: smtpUsername || undefined,
        smtp_password: smtpPassword || undefined, smtp_tls: true,
      } : { name, channel_type: type, webhook_url: destination });
      setName(''); setDestination(''); setSmtpPassword(''); await onChange();
    } catch (error) { window.alert(error instanceof Error ? error.message : 'Could not create channel'); }
    finally { setSaving(false); }
  };

  return <div className="grid gap-6 xl:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
    <form onSubmit={submit} className="space-y-4 rounded-2xl border border-border/70 bg-background/25 p-5">
      <h3 className="font-semibold">Add destination</h3>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Name"><input required value={name} onChange={e => setName(e.target.value)} className="input" placeholder="On-call Slack" /></Field>
        <Field label="Type"><select value={type} onChange={e => setType(e.target.value as NotificationChannelType)} className="input"><option value="generic_webhook">Generic webhook</option><option value="slack">Slack</option><option value="discord">Discord</option><option value="email">Email</option></select></Field>
      </div>
      <Field label={type === 'email' ? 'Recipient email' : 'HTTPS webhook URL'}><input required type={type === 'email' ? 'email' : 'url'} value={destination} onChange={e => setDestination(e.target.value)} className="input" placeholder={type === 'email' ? 'oncall@example.com' : 'https://…'} /></Field>
      {type === 'email' && <>
        <div className="grid grid-cols-[1fr_100px] gap-3"><Field label="SMTP host"><input required value={smtpHost} onChange={e => setSmtpHost(e.target.value)} className="input" /></Field><Field label="Port"><input required type="number" value={smtpPort} onChange={e => setSmtpPort(Number(e.target.value))} className="input" /></Field></div>
        <Field label="Sender"><input required type="email" value={smtpFrom} onChange={e => setSmtpFrom(e.target.value)} className="input" placeholder="watchtower@example.com" /></Field>
        <div className="grid grid-cols-2 gap-3"><Field label="SMTP username"><input value={smtpUsername} onChange={e => setSmtpUsername(e.target.value)} className="input" /></Field><Field label="SMTP password"><input type="password" value={smtpPassword} onChange={e => setSmtpPassword(e.target.value)} className="input" /></Field></div>
      </>}
      <button disabled={saving} className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground disabled:opacity-50"><Plus className="h-4 w-4" />{saving ? 'Saving…' : 'Add channel'}</button>
    </form>
    <div className="space-y-3">
      {channels.length === 0 && <Empty text="No destinations configured. Alerts will remain visible in WatchTower but will not be sent externally." />}
      {channels.map(channel => <div key={channel.id} className="flex flex-col gap-4 rounded-2xl border border-border/70 bg-background/25 p-4 sm:flex-row sm:items-center sm:justify-between">
        <div><div className="flex items-center gap-2"><span className={`h-2 w-2 rounded-full ${channel.enabled ? 'bg-emerald-400' : 'bg-slate-500'}`} /><p className="font-medium">{channel.name}</p></div><p className="mt-1 text-xs text-muted-foreground">{channel.channel_type.replace('_', ' ')} · {channel.destination}</p></div>
        <div className="flex gap-2">
          <button onClick={async () => { await testNotificationChannel(channel.id); window.alert('Test queued. Check Delivery history for its result.'); await onChange(); }} className="rounded-lg border border-border p-2 hover:bg-white/5" title="Send test"><FlaskConical className="h-4 w-4" /></button>
          <button onClick={async () => { await setNotificationChannelEnabled(channel.id, !channel.enabled); await onChange(); }} className="rounded-lg border border-border px-3 py-2 text-xs">{channel.enabled ? 'Disable' : 'Enable'}</button>
          <button onClick={async () => { if (confirm(`Delete ${channel.name}?`)) { await deleteNotificationChannel(channel.id); await onChange(); } }} className="rounded-lg border border-border p-2 text-red-300 hover:bg-red-400/10" title="Delete"><Trash2 className="h-4 w-4" /></button>
        </div>
      </div>)}
    </div>
  </div>;
}

function MaintenancePanel({ rules, silences, onChange }: { rules: AlertRule[]; silences: AlertSilence[]; onChange: () => Promise<void> }) {
  const now = new Date(); const later = new Date(now.getTime() + 60 * 60 * 1000);
  const local = (date: Date) => new Date(date.getTime() - date.getTimezoneOffset() * 60_000).toISOString().slice(0, 16);
  const [form, setForm] = useState({ name: '', reason: '', rule_id: '', agent_id: '', starts_at: local(now), ends_at: local(later) });
  const submit = async (event: React.FormEvent) => { event.preventDefault(); try { await createAlertSilence({ ...form, rule_id: form.rule_id || undefined, agent_id: form.agent_id || undefined, starts_at: new Date(form.starts_at).toISOString(), ends_at: new Date(form.ends_at).toISOString(), created_by: localStorage.getItem('username') || 'admin' }); setForm(previous => ({ ...previous, name: '', reason: '' })); await onChange(); } catch (error) { window.alert(error instanceof Error ? error.message : 'Could not schedule silence'); } };
  return <div className="grid gap-6 xl:grid-cols-[minmax(0,0.85fr)_minmax(0,1.15fr)]">
    <form onSubmit={submit} className="space-y-4 rounded-2xl border border-border/70 bg-background/25 p-5"><h3 className="font-semibold">Schedule maintenance</h3><Field label="Window name"><input required value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} className="input" placeholder="Database upgrade" /></Field><Field label="Reason"><input value={form.reason} onChange={e => setForm({ ...form, reason: e.target.value })} className="input" /></Field><div className="grid grid-cols-2 gap-3"><Field label="Starts"><input required type="datetime-local" value={form.starts_at} onChange={e => setForm({ ...form, starts_at: e.target.value })} className="input" /></Field><Field label="Ends"><input required type="datetime-local" value={form.ends_at} onChange={e => setForm({ ...form, ends_at: e.target.value })} className="input" /></Field></div><div className="grid grid-cols-2 gap-3"><Field label="Rule scope"><select value={form.rule_id} onChange={e => setForm({ ...form, rule_id: e.target.value })} className="input"><option value="">All rules</option>{rules.map(rule => <option key={rule.id} value={rule.id}>{rule.name}</option>)}</select></Field><Field label="Agent scope"><input value={form.agent_id} onChange={e => setForm({ ...form, agent_id: e.target.value })} className="input" placeholder="All agents" /></Field></div><button className="inline-flex items-center gap-2 rounded-xl bg-primary px-4 py-2 text-sm font-medium text-primary-foreground"><CalendarClock className="h-4 w-4" />Schedule silence</button></form>
    <div className="space-y-3">{silences.length === 0 && <Empty text="No maintenance windows scheduled." />}{silences.map(silence => { const active = new Date(silence.starts_at) <= new Date() && new Date(silence.ends_at) > new Date(); return <div key={silence.id} className="rounded-2xl border border-border/70 bg-background/25 p-4"><div className="flex items-start justify-between gap-3"><div><div className="flex items-center gap-2"><p className="font-medium">{silence.name}</p>{active && <span className="rounded-full bg-amber-400/10 px-2 py-0.5 text-[10px] font-semibold uppercase text-amber-300">active</span>}</div><p className="mt-1 text-xs text-muted-foreground">{new Date(silence.starts_at).toLocaleString()} → {new Date(silence.ends_at).toLocaleString()}</p><p className="mt-2 text-sm text-muted-foreground">{silence.reason || 'No reason provided'} · {silence.agent_id || 'all agents'}</p></div><button onClick={async () => { await deleteAlertSilence(silence.id); await onChange(); }} className="rounded-lg border border-border p-2 text-red-300 hover:bg-red-400/10"><Trash2 className="h-4 w-4" /></button></div></div>; })}</div>
  </div>;
}

function DeliveryPanel({ deliveries }: { deliveries: NotificationDelivery[] }) {
  return <div className="overflow-hidden rounded-2xl border border-border/70"><div className="overflow-x-auto"><table className="w-full text-left text-sm"><thead className="bg-background/60 text-xs uppercase tracking-wide text-muted-foreground"><tr><th className="px-4 py-3">Status</th><th className="px-4 py-3">Channel</th><th className="px-4 py-3">Event</th><th className="px-4 py-3">Attempts</th><th className="px-4 py-3">Time</th><th className="px-4 py-3">Result</th></tr></thead><tbody className="divide-y divide-border/70">{deliveries.map(delivery => <tr key={delivery.id}><td className="px-4 py-3"><span className={`inline-flex items-center gap-1.5 ${delivery.status === 'delivered' ? 'text-emerald-300' : delivery.status === 'failed' ? 'text-red-300' : 'text-amber-300'}`}>{delivery.status === 'delivered' ? <CheckCircle2 className="h-4 w-4" /> : delivery.status === 'failed' ? <XCircle className="h-4 w-4" /> : <Send className="h-4 w-4" />}{delivery.status}</span></td><td className="px-4 py-3 font-medium">{delivery.channel_name}</td><td className="px-4 py-3 capitalize">{delivery.event_type}</td><td className="px-4 py-3 tabular-nums">{delivery.attempt_count}/{delivery.max_attempts}</td><td className="px-4 py-3 text-muted-foreground">{new Date(delivery.created_at).toLocaleString()}</td><td className="max-w-[320px] truncate px-4 py-3 text-muted-foreground" title={delivery.error_message}>{delivery.response_status ? `HTTP ${delivery.response_status}` : delivery.error_message || 'Queued'}</td></tr>)}</tbody></table></div>{deliveries.length === 0 && <Empty text="No notification attempts recorded yet." />}</div>;
}

function Field({ label, children }: { label: string; children: React.ReactNode }) { return <label className="block text-xs font-medium text-muted-foreground">{label}<span className="mt-1.5 block [&_.input]:w-full [&_.input]:rounded-xl [&_.input]:border [&_.input]:border-border [&_.input]:bg-background/60 [&_.input]:px-3 [&_.input]:py-2.5 [&_.input]:text-sm [&_.input]:text-foreground [&_.input]:outline-none [&_.input]:focus:border-primary">{children}</span></label>; }
function Empty({ text }: { text: string }) { return <div className="rounded-2xl border border-dashed border-border p-8 text-center text-sm text-muted-foreground">{text}</div>; }

"use client";

import { useEffect, useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import {
  ArrowLeft,
  AlertTriangle,
  BellRing,
  Check,
  Clock3,
  Info,
  Loader2,
  ShieldAlert,
  SlidersHorizontal,
} from "lucide-react";
import {
  createAlertRule,
  getNotificationChannels,
  type AlertRule,
  type NotificationChannel,
} from "@/lib/alerts-api";

const METRICS = [
  ["cpu_usage", "CPU utilization", "%"],
  ["memory_usage", "Memory utilization", "%"],
  ["disk_usage", "Disk utilization", "%"],
  ["inode_max_usage", "Inode utilization", "%"],
  ["load_1", "Load average · 1 minute", ""],
  ["cpu_iowait_percent", "CPU I/O wait", "%"],
  ["cpu_steal_percent", "CPU steal time", "%"],
  ["swap_usage", "Swap utilization", "%"],
  ["oom_kills_delta", "New OOM kills", "events"],
  ["network_rx_bytes_per_sec", "Network receive rate", "bytes/s"],
  ["network_tx_bytes_per_sec", "Network transmit rate", "bytes/s"],
  ["tcp_connections_total", "TCP connections", "connections"],
  ["agent_queue_records", "Pending agent samples", "samples"],
  ["agent_dropped_samples", "Agent dropped samples", "samples"],
  ["db_connections_active", "Database active connections", "connections"],
  ["db_cache_hit_ratio", "Database cache hit ratio", "%"],
  ["db_slow_queries", "Database slow queries", "queries"],
  ["db_locks_waiting", "Database waiting locks", "locks"],
] as const;
const CONDITIONS: {
  value: AlertRule["condition"];
  label: string;
  symbol: string;
}[] = [
  { value: "greater_than", label: "Greater than", symbol: ">" },
  { value: "less_than", label: "Less than", symbol: "<" },
  { value: "equals", label: "Equal to", symbol: "=" },
  { value: "not_equals", label: "Not equal to", symbol: "≠" },
];
const SEVERITIES = [
  {
    value: "info",
    label: "Informational",
    description: "Track a change without urgent action.",
    icon: Info,
    color: "text-cyan-300",
  },
  {
    value: "warning",
    label: "Warning",
    description: "Investigate before service is affected.",
    icon: AlertTriangle,
    color: "text-amber-300",
  },
  {
    value: "critical",
    label: "Critical",
    description: "Immediate operator attention required.",
    icon: ShieldAlert,
    color: "text-rose-300",
  },
] as const;
const inputClass =
  "w-full rounded-xl border border-white/10 bg-[#0c121d] px-3.5 py-2.5 text-sm text-slate-100 outline-none transition focus:border-cyan-400/60 focus:ring-2 focus:ring-cyan-400/10 disabled:opacity-50";
type RuleDraft = Omit<AlertRule, "id" | "created_at" | "enabled">;

export default function NewAlertRulePage() {
  const router = useRouter();
  const [saving, setSaving] = useState(false);
  const [customMetric, setCustomMetric] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [channels, setChannels] = useState<NotificationChannel[]>([]);
  const [channelsLoading, setChannelsLoading] = useState(true);
  const [channelsError, setChannelsError] = useState(false);
  const [form, setForm] = useState<RuleDraft>({
    name: "",
    description: "",
    metric: "cpu_usage",
    condition: "greater_than",
    threshold: 80,
    duration_seconds: 60,
    severity: "warning",
    channels: [],
    cooldown_seconds: 300,
    agent_filter: "",
  });
  useEffect(() => {
    let active = true;
    getNotificationChannels()
      .then((result) => {
        if (active) setChannels(result);
      })
      .catch(() => {
        if (active) setChannelsError(true);
      })
      .finally(() => {
        if (active) setChannelsLoading(false);
      });
    return () => {
      active = false;
    };
  }, []);
  const metric = METRICS.find(([value]) => value === form.metric);
  const unit = metric?.[2] ?? "";
  const condition = CONDITIONS.find((item) => item.value === form.condition)!;
  const destinationCount = form.channels.length;
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (
      !form.name.trim() ||
      !Number.isFinite(form.threshold) ||
      !Number.isInteger(form.duration_seconds) ||
      !Number.isInteger(form.cooldown_seconds) ||
      form.duration_seconds < 0 ||
      form.cooldown_seconds < 0
    ) {
      setError(
        "Enter a rule name, a finite threshold, and valid whole-second timings.",
      );
      return;
    }
    if (unit === "%" && (form.threshold < 0 || form.threshold > 100)) {
      setError("Percentage thresholds must be between 0 and 100.");
      return;
    }
    setSaving(true);
    try {
      await createAlertRule({
        ...form,
        name: form.name.trim(),
        description: form.description?.trim(),
        agent_filter: form.agent_filter?.trim() || undefined,
      });
      router.push("/alerts");
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Could not create this rule. Your inputs have been preserved.",
      );
      setSaving(false);
    }
  }
  return (
    <div className="page-shell max-w-[1400px]">
      <Link
        href="/alerts"
        className="mb-6 inline-flex items-center gap-2 text-sm text-slate-400 transition hover:text-white"
      >
        <ArrowLeft className="h-4 w-4" />
        Alert policies
      </Link>
      <div className="mb-8 flex items-start gap-4">
        <div className="rounded-2xl border border-cyan-400/20 bg-cyan-400/5 p-3">
          <SlidersHorizontal className="h-6 w-6 text-cyan-300" />
        </div>
        <div>
          <p className="eyebrow mb-2">Policy configuration</p>
          <h1 className="text-3xl font-semibold tracking-tight text-white">
            Create alert rule
          </h1>
          <p className="mt-2 text-sm text-slate-400">
            Define a precise signal, reduce noise, and route incidents to the
            right operators.
          </p>
        </div>
      </div>
      <form
        onSubmit={submit}
        className="grid items-start gap-6 xl:grid-cols-[minmax(0,1fr)_320px]"
      >
        <fieldset disabled={saving} className="min-w-0 space-y-6">
          {error && (
            <div
              role="alert"
              className="flex items-start gap-3 rounded-xl border border-rose-400/25 bg-rose-400/5 p-4 text-sm text-rose-200"
            >
              <AlertTriangle className="h-5 w-5 shrink-0" />
              {error}
            </div>
          )}
          <section className="surface-panel p-5 sm:p-7">
            <div className="mb-6 flex items-center gap-3">
              <span className="rounded-lg bg-white/5 px-2.5 py-1 text-xs text-slate-400">
                01
              </span>
              <h2 className="font-semibold text-white">Policy identity</h2>
            </div>
            <div className="space-y-5">
              <div>
                <label
                  htmlFor="rule-name"
                  className="mb-2 block text-sm font-medium"
                >
                  Rule name <span className="text-slate-500">· required</span>
                </label>
                <input
                  id="rule-name"
                  required
                  maxLength={128}
                  value={form.name}
                  onChange={(event) =>
                    setForm({ ...form, name: event.target.value })
                  }
                  className={inputClass}
                  placeholder="Production host · sustained CPU pressure"
                  autoComplete="off"
                />
              </div>
              <div>
                <label
                  htmlFor="rule-description"
                  className="mb-2 block text-sm font-medium"
                >
                  Operator context
                </label>
                <textarea
                  id="rule-description"
                  maxLength={2000}
                  rows={3}
                  value={form.description}
                  onChange={(event) =>
                    setForm({ ...form, description: event.target.value })
                  }
                  className={inputClass}
                  placeholder="Explain the impact and the first investigation step."
                />
              </div>
              <div>
                <label
                  htmlFor="rule-agent"
                  className="mb-2 block text-sm font-medium"
                >
                  Agent scope
                </label>
                <input
                  id="rule-agent"
                  maxLength={128}
                  pattern="[A-Za-z0-9_.:\-]+"
                  value={form.agent_filter}
                  onChange={(event) =>
                    setForm({ ...form, agent_filter: event.target.value })
                  }
                  className={inputClass}
                  placeholder="All agents owned by your API key"
                  aria-describedby="scope-help"
                />
                <p id="scope-help" className="mt-2 text-xs text-slate-500">
                  Leave empty for all agents owned by your enterprise API key, or enter one of your exact agent IDs.
                </p>
              </div>
            </div>
          </section>
          <section className="surface-panel p-5 sm:p-7">
            <div className="mb-6 flex items-center gap-3">
              <span className="rounded-lg bg-white/5 px-2.5 py-1 text-xs text-slate-400">
                02
              </span>
              <h2 className="font-semibold text-white">Trigger and timing</h2>
            </div>
            <div className="grid gap-5 sm:grid-cols-2">
              <div>
                <label
                  htmlFor="rule-metric"
                  className="mb-2 block text-sm font-medium"
                >
                  Metric
                </label>
                <select
                  id="rule-metric"
                  className={inputClass}
                  value={customMetric ? "custom" : form.metric}
                  onChange={(event) => {
                    setCustomMetric(event.target.value === "custom");
                    setForm({
                      ...form,
                      metric:
                        event.target.value === "custom"
                          ? ""
                          : event.target.value,
                    });
                  }}
                >
                  {METRICS.map(([value, label, unit]) => (
                    <option key={value} value={value}>
                      {label}
                      {unit ? ` (${unit})` : ""}
                    </option>
                  ))}
                  <option value="custom">Custom metric</option>
                </select>
                {customMetric && (
                  <div className="mt-3">
                    <label
                      htmlFor="custom-metric"
                      className="mb-2 block text-xs text-slate-400"
                    >
                      Exact metric identifier
                    </label>
                    <input
                      id="custom-metric"
                      required
                      maxLength={128}
                      pattern="[A-Za-z0-9_.:]+(?:-[A-Za-z0-9_.:]+)*"
                      value={form.metric}
                      onChange={(event) =>
                        setForm({ ...form, metric: event.target.value })
                      }
                      className={inputClass}
                      placeholder="process_postgres_cpu_usage"
                    />
                  </div>
                )}
              </div>
              <div>
                <label
                  htmlFor="rule-condition"
                  className="mb-2 block text-sm font-medium"
                >
                  Comparison
                </label>
                <select
                  id="rule-condition"
                  className={inputClass}
                  value={form.condition}
                  onChange={(event) =>
                    setForm({
                      ...form,
                      condition: event.target.value as AlertRule["condition"],
                    })
                  }
                >
                  {CONDITIONS.map((item) => (
                    <option key={item.value} value={item.value}>
                      {item.label} ({item.symbol})
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label
                  htmlFor="rule-threshold"
                  className="mb-2 block text-sm font-medium"
                >
                  Threshold{unit ? ` · ${unit}` : ""}
                </label>
                <input
                  id="rule-threshold"
                  type="number"
                  required
                  step="any"
                  min={0}
                  max={unit === "%" ? 100 : undefined}
                  value={Number.isNaN(form.threshold) ? "" : form.threshold}
                  onChange={(event) =>
                    setForm({ ...form, threshold: event.target.valueAsNumber })
                  }
                  className={inputClass}
                />
              </div>
              <div>
                <label
                  htmlFor="rule-duration"
                  className="mb-2 block text-sm font-medium"
                >
                  Sustained duration · seconds
                </label>
                <input
                  id="rule-duration"
                  type="number"
                  required
                  min={0}
                  max={86400}
                  step={1}
                  value={
                    Number.isNaN(form.duration_seconds)
                      ? ""
                      : form.duration_seconds
                  }
                  onChange={(event) =>
                    setForm({
                      ...form,
                      duration_seconds: event.target.valueAsNumber,
                    })
                  }
                  className={inputClass}
                  aria-describedby="duration-help"
                />
                <p id="duration-help" className="mt-2 text-xs text-slate-500">
                  Avoid transient spikes. Zero fires on the first matching
                  evaluation.
                </p>
              </div>
            </div>
          </section>
          <section className="surface-panel p-5 sm:p-7">
            <div className="mb-6 flex items-center gap-3">
              <span className="rounded-lg bg-white/5 px-2.5 py-1 text-xs text-slate-400">
                03
              </span>
              <h2 className="font-semibold text-white">Severity and routing</h2>
            </div>
            <fieldset>
              <legend className="mb-3 text-sm font-medium">
                Incident severity
              </legend>
              <div className="grid gap-3 sm:grid-cols-3">
                {SEVERITIES.map(
                  ({ value, label, description, icon: Icon, color }) => (
                    <label
                      key={value}
                      className={`relative cursor-pointer rounded-xl border p-4 transition focus-within:ring-2 focus-within:ring-cyan-300 ${form.severity === value ? "border-cyan-400/50 bg-cyan-400/5" : "border-white/10 hover:border-white/25"}`}
                    >
                      <input
                        type="radio"
                        name="severity"
                        value={value}
                        checked={form.severity === value}
                        onChange={() => setForm({ ...form, severity: value })}
                        className="sr-only"
                      />
                      <div className="mb-3 flex justify-between">
                        <Icon className={`h-5 w-5 ${color}`} />
                        {form.severity === value && (
                          <Check className="h-4 w-4 text-cyan-300" />
                        )}
                      </div>
                      <span className="block text-sm font-semibold">
                        {label}
                      </span>
                      <span className="mt-1 block text-xs leading-relaxed text-slate-500">
                        {description}
                      </span>
                    </label>
                  ),
                )}
              </div>
            </fieldset>
            <fieldset className="mt-6">
              <legend className="mb-3 text-sm font-medium">
                Notification destinations
              </legend>
              {channelsLoading ? (
                <p role="status" className="text-sm text-slate-400">
                  Loading configured destinations…
                </p>
              ) : channelsError ? (
                <p role="alert" className="text-sm text-amber-300">
                  Destinations could not be loaded. Refresh before configuring
                  delivery.
                </p>
              ) : (
                <div className="grid gap-3 sm:grid-cols-2">
                  {channels
                    .filter((channel) => channel.enabled && channel.configured)
                    .map((channel) => (
                      <label
                        key={channel.id}
                        className="flex cursor-pointer items-center gap-3 rounded-xl border border-white/10 p-3.5 hover:border-white/25"
                      >
                        <input
                          type="checkbox"
                          className="h-4 w-4 accent-cyan-400"
                          checked={form.channels.includes(channel.id)}
                          onChange={(event) =>
                            setForm((previous) => ({
                              ...previous,
                              channels: event.target.checked
                                ? [...previous.channels, channel.id]
                                : previous.channels.filter(
                                    (id) => id !== channel.id,
                                  ),
                            }))
                          }
                        />
                        <span>
                          <span className="block text-sm font-medium">
                            {channel.name}
                          </span>
                          <span className="text-xs capitalize text-slate-500">
                            {channel.channel_type.replaceAll("_", " ")}
                          </span>
                        </span>
                      </label>
                    ))}
                </div>
              )}
              {!channelsLoading &&
                !channelsError &&
                channels.filter(
                  (channel) => channel.enabled && channel.configured,
                ).length === 0 && (
                  <p className="text-sm text-slate-400">
                    No active destinations.{" "}
                    <Link href="/alerts" className="text-cyan-300 underline">
                      Configure a channel
                    </Link>{" "}
                    to enable notifications.
                  </p>
                )}
            </fieldset>
            <div className="mt-6">
              <label
                htmlFor="rule-cooldown"
                className="mb-2 block text-sm font-medium"
              >
                Repeat notification cooldown · seconds
              </label>
              <input
                id="rule-cooldown"
                type="number"
                required
                min={0}
                max={86400}
                step={1}
                value={
                  Number.isNaN(form.cooldown_seconds)
                    ? ""
                    : form.cooldown_seconds
                }
                onChange={(event) =>
                  setForm({
                    ...form,
                    cooldown_seconds: event.target.valueAsNumber,
                  })
                }
                className={inputClass}
              />
              <p className="mt-2 text-xs text-slate-500">
                Minimum interval between repeated notifications for the same
                incident.
              </p>
            </div>
          </section>
        </fieldset>
        <aside className="surface-panel p-6 xl:sticky xl:top-28">
          <div className="flex items-center gap-2 text-sm font-semibold">
            <BellRing className="h-4 w-4 text-cyan-300" />
            Rule preview
          </div>
          <p className="mt-5 break-words text-lg font-semibold text-white">
            {form.name.trim() || "Untitled policy"}
          </p>
          <div className="my-5 rounded-xl border border-white/10 bg-black/15 p-4">
            <p className="text-xs text-slate-500">Firing condition</p>
            <p className="mt-2 text-sm text-slate-200">
              {metric?.[1] || form.metric || "Custom metric"}{" "}
              <span className="font-mono text-cyan-300">
                {condition.symbol}{" "}
                {Number.isFinite(form.threshold) ? form.threshold : "—"}
                {unit ? ` ${unit}` : ""}
              </span>
            </p>
            <p className="mt-3 flex items-center gap-2 text-xs text-slate-400">
              <Clock3 className="h-3.5 w-3.5" />
              Sustained for{" "}
              {Number.isFinite(form.duration_seconds)
                ? form.duration_seconds
                : "—"}{" "}
              seconds
            </p>
          </div>
          <dl className="space-y-3 text-sm">
            <div className="flex justify-between gap-4">
              <dt className="text-slate-500">Scope</dt>
              <dd className="break-all text-right">
                {form.agent_filter?.trim() || "All agents"}
              </dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Severity</dt>
              <dd className="capitalize">{form.severity}</dd>
            </div>
            <div className="flex justify-between">
              <dt className="text-slate-500">Destinations</dt>
              <dd>{destinationCount}</dd>
            </div>
          </dl>
          {destinationCount === 0 && (
            <p className="mt-5 rounded-xl border border-amber-400/20 bg-amber-400/5 p-3 text-xs leading-relaxed text-amber-200">
              This rule will create dashboard incidents without external
              notifications.
            </p>
          )}
          <div className="mt-6 border-t border-white/10 pt-5">
            <button
              type="submit"
              disabled={saving}
              className="flex w-full items-center justify-center gap-2 rounded-xl bg-cyan-400 px-4 py-3 text-sm font-semibold text-slate-950 transition hover:bg-cyan-300 disabled:cursor-wait disabled:opacity-60"
            >
              {saving ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Check className="h-4 w-4" />
              )}
              {saving ? "Creating policy…" : "Create alert rule"}
            </button>
            <Link
              href="/alerts"
              className="mt-3 block rounded-xl py-2 text-center text-sm text-slate-400 hover:text-white"
            >
              Cancel
            </Link>
          </div>
        </aside>
      </form>
    </div>
  );
}

"use client";
import { useEffect, useState } from "react";
import { HardDrive, RefreshCw, ShieldCheck } from "lucide-react";
import { getAuthHeaders } from "@/lib/auth-utils";

interface DeliveryStatus {
  last_seen: string;
  last_heartbeat_at: string | null;
  last_successful_upload: string | null;
  report: {
    version?: string;
    architecture?: string;
    os?: string;
    capabilities?: string[];
    queue_records?: number;
    queue_bytes?: number;
    dropped_samples?: number;
    config_revision?: number;
  };
}
export default function AgentDeliveryPanel({ agentId }: { agentId: string }) {
  const [status, setStatus] = useState<DeliveryStatus | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    const controller = new AbortController();
    let timer: ReturnType<typeof setTimeout>;
    async function update() {
      try {
        const response = await fetch(
          `${process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080"}/api/v1/agents/${encodeURIComponent(agentId)}/delivery`,
          {
            headers: getAuthHeaders(),
            signal: controller.signal,
            cache: "no-store",
          },
        );
        if (!response.ok) throw new Error("Delivery status unavailable");
        const value: DeliveryStatus = await response.json();
        if (!controller.signal.aborted) {
          setStatus(value);
          setError(false);
        }
      } catch {
        if (!controller.signal.aborted) setError(true);
      } finally {
        if (!controller.signal.aborted) timer = setTimeout(update, 10000);
      }
    }
    setStatus(null);
    setError(false);
    void update();
    return () => {
      controller.abort();
      clearTimeout(timer);
    };
  }, [agentId]);
  const report = status?.report;
  return (
    <section className="surface-panel p-5 sm:p-6">
      <div className="mb-5 flex items-center justify-between gap-4">
        <div>
          <p className="eyebrow mb-1">Agent reliability</p>
          <h2 className="text-lg font-semibold">Delivery and identity</h2>
        </div>
        <ShieldCheck className="h-5 w-5 text-cyan-300" />
      </div>
      {error && (
        <p role="status" className="mb-4 text-xs text-amber-300">
          Delivery status could not be refreshed
          {status ? "; showing last known information." : "."}
        </p>
      )}
      {!status && !error ? (
        <p className="text-sm text-slate-500">Loading delivery status…</p>
      ) : status ? (
        <>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <div>
              <p className="text-xs text-slate-500">Agent build</p>
              <p className="mt-1 font-mono text-sm">
                {report?.version || "Not reported"} ·{" "}
                {report?.architecture || "—"}
              </p>
              <p className="mt-1 text-xs text-slate-400">
                {report?.os || "OS not reported"}
              </p>
            </div>
            <div>
              <p className="flex items-center gap-1.5 text-xs text-slate-500">
                <HardDrive className="h-3.5 w-3.5" />
                Disk queue
              </p>
              <p className="mt-1 text-sm">
                {report?.queue_records ?? "—"} pending samples
              </p>
              <p className="mt-1 text-xs text-slate-400">
                {report?.queue_bytes !== undefined
                  ? (report.queue_bytes / 1048576).toFixed(2)
                  : "—"}{" "}
                MiB · {report?.dropped_samples ?? 0} rejected
              </p>
            </div>
            <div>
              <p className="flex items-center gap-1.5 text-xs text-slate-500">
                <RefreshCw className="h-3.5 w-3.5" />
                Last durable upload
              </p>
              <p className="mt-1 text-sm">
                {status.last_successful_upload
                  ? new Date(status.last_successful_upload).toLocaleTimeString()
                  : "No confirmed upload"}
              </p>
              <p className="mt-1 text-xs text-slate-400">
                Heartbeat:{" "}
                {status.last_heartbeat_at
                  ? new Date(status.last_heartbeat_at).toLocaleTimeString()
                  : "Not reported"}
              </p>
            </div>
            <div>
              <p className="text-xs text-slate-500">Signed configuration</p>
              <p className="mt-1 text-sm">
                Revision {report?.config_revision ?? 0}
              </p>
              <p className="mt-1 text-xs text-slate-400">
                Read-only process exploration
              </p>
            </div>
          </div>
          {report?.capabilities && (
            <div className="mt-5 flex flex-wrap gap-2 border-t border-white/10 pt-4">
              {report.capabilities.map((capability) => (
                <span
                  key={capability}
                  className="rounded-lg border border-white/10 px-2 py-1 text-[11px] text-slate-400"
                >
                  {capability.replaceAll("_", " ")}
                </span>
              ))}
            </div>
          )}
        </>
      ) : null}
    </section>
  );
}

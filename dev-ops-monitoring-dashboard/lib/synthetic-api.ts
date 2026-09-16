import { getAuthHeaders } from './auth-utils';
export type CheckKind = 'http' | 'tcp' | 'dns' | 'tls';
export interface CheckSpec {
  name: string; kind: CheckKind; target: string; port?: number;
  interval_seconds: number; timeout_seconds: number; failure_threshold: number;
  expected_status: number; expected_content?: string; dns_record_type: 'A' | 'AAAA';
  expected_dns_value?: string; tls_expiry_days: number; channels: string[];
}
export interface ProbeResult {
  checked_at: string; success: boolean; response_time_ms: number;
  status_code: number | null; content_matched: boolean | null; resolved_addresses: string[];
  tls_expires_at: string | null; tls_days_remaining: number | null; error: string | null;
}
export interface SyntheticCheck {
  id: string; agent_id: string; spec: CheckSpec; enabled: boolean;
  consecutive_failures: number; active_alert_id: string | null;
  last_result: ProbeResult | null; next_run_at: string; created_at: string;
}
export interface CheckHistory { id: number; result: ProbeResult }
const base = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
async function request<T>(path: string, method = 'GET', body?: unknown): Promise<T> {
  const response = await fetch(`${base}/api/v1/synthetic-checks${path}`, {
    method, headers: { ...getAuthHeaders(), 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body), cache: 'no-store',
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.error || error.message || `Request failed (${response.status})`);
  }
  return response.status === 204 ? undefined as T : response.json();
}
export const listSyntheticChecks = () => request<SyntheticCheck[]>('');
export const createSyntheticCheck = (spec: CheckSpec) => request<{ id: string }>('', 'POST', spec);
export const setSyntheticEnabled = (id: string, enabled: boolean) => request<void>(`/${id}`, 'PUT', { enabled });
export const deleteSyntheticCheck = (id: string) => request<void>(`/${id}`, 'DELETE');
export const getSyntheticHistory = (id: string) => request<CheckHistory[]>(`/${id}/history?limit=100`);

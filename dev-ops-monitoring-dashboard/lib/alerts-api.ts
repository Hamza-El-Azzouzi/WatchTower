import { getAuthHeaders } from './auth-utils';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface AlertRule {
  id: string;
  name: string;
  description?: string;
  metric: string;
  condition: 'greater_than' | 'less_than' | 'equals' | 'not_equals';
  threshold: number;
  threshold_percent?: number;
  duration_seconds: number;
  severity: 'info' | 'warning' | 'critical';
  channels: string[];
  enabled: boolean;
  agent_filter?: string;
  created_at: string;
  cooldown_seconds: number;
}

export interface Alert {
  id: string;
  rule_id: string;
  rule_name: string;
  agent_id: string;
  agent_name: string;
  state: 'pending' | 'firing' | 'resolved';
  metric: string;
  current_value: number;
  threshold: number;
  condition: 'greater_than' | 'less_than' | 'equals' | 'not_equals';
  severity: 'info' | 'warning' | 'critical';
  message: string;
  triggered_at: string;
  resolved_at?: string;
  acknowledged: boolean;
  acknowledged_at?: string;
  acknowledged_by?: string;
  last_notification_at?: string;
}

export interface AlertsResponse {
  active_alerts: Alert[];
  recent_alerts: Alert[];
  total_active: number;
}

export interface AlertRulesResponse {
  rules: AlertRule[];
  total: number;
}

export type NotificationChannelType = 'generic_webhook' | 'slack' | 'discord' | 'email';

export interface NotificationChannel {
  id: string;
  name: string;
  channel_type: NotificationChannelType;
  destination: string;
  enabled: boolean;
  configured: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateNotificationChannel {
  name: string;
  channel_type: NotificationChannelType;
  webhook_url?: string;
  email_to?: string;
  smtp_host?: string;
  smtp_port?: number;
  smtp_username?: string;
  smtp_password?: string;
  smtp_from?: string;
  smtp_tls?: boolean;
}

export interface AlertSilence {
  id: string;
  name: string;
  reason?: string;
  rule_id?: string;
  agent_id?: string;
  starts_at: string;
  ends_at: string;
  created_by: string;
  created_at: string;
}

export interface NotificationDelivery {
  id: number;
  alert_id: string;
  channel_id?: string;
  channel_name: string;
  event_type: 'firing' | 'resolved' | 'test';
  status: 'pending' | 'delivered' | 'failed';
  attempt_count: number;
  response_status?: number;
  error_message?: string;
  created_at: string;
  delivered_at?: string;
  next_attempt_at: string;
  last_attempt_at?: string;
  max_attempts: number;
}

export interface IncidentEvent {
  event_id: string;
  alert_id?: string;
  rule_id?: string;
  agent_id: string;
  event_type: 'pending' | 'firing' | 'recovery' | 'acknowledged' | 'process_spike' | 'log_error';
  severity: 'info' | 'warning' | 'critical';
  title: string;
  description: string;
  metadata: Record<string, unknown>;
  occurred_at: string;
}

export async function getAlerts(): Promise<AlertsResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alerts`, {
    headers: getAuthHeaders()
  });
  if (!response.ok) {
    throw new Error('Failed to fetch alerts');
  }
  return response.json();
}

export async function getAlertRules(): Promise<AlertRulesResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alert-rules`, {
    headers: getAuthHeaders()
  });
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to fetch alert rules' }));
    throw new Error(errorData.error || 'Failed to fetch alert rules');
  }
  return response.json();
}

export async function getAlertRule(ruleId: string): Promise<AlertRule> {
  return apiRequest<AlertRule>(`/api/v1/alert-rules/${encodeURIComponent(ruleId)}`);
}

export async function getEffectiveAlertRules(agentId: string): Promise<AlertRulesResponse> {
  return apiRequest(`/api/v1/alert-rules/effective?agent_id=${encodeURIComponent(agentId)}`);
}

export async function createAlertRule(rule: Omit<AlertRule, 'id' | 'created_at' | 'enabled'>): Promise<AlertRule> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alert-rules`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders()
    },
    body: JSON.stringify(rule),
  });
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to create alert rule' }));
    throw new Error(errorData.error || 'Failed to create alert rule');
  }
  
  return response.json();
}

export async function deleteAlertRule(ruleId: string): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alert-rules/${ruleId}`, {
    method: 'DELETE',
    headers: getAuthHeaders()
  });
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to delete alert rule' }));
    throw new Error(errorData.error || 'Failed to delete alert rule');
  }
}

export async function updateAlertRule(ruleId: string, rule: Partial<Omit<AlertRule, 'id' | 'created_at' | 'enabled'>>): Promise<AlertRule> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alert-rules/${ruleId}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders()
    },
    body: JSON.stringify(rule),
  });
  
  if (!response.ok) {
    const errorData = await response.json();
    throw new Error(errorData.error || 'Failed to update alert rule');
  }
  
  return response.json();
}

export async function toggleAlertRule(ruleId: string): Promise<AlertRule> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alert-rules/${ruleId}/toggle`, {
    method: 'POST',
    headers: getAuthHeaders()
  });
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to toggle alert rule' }));
    throw new Error(errorData.error || 'Failed to toggle alert rule');
  }
  
  return response.json();
}

export async function acknowledgeAlert(alertId: string, acknowledgedBy: string): Promise<Alert> {
  const response = await fetch(`${API_BASE_URL}/api/v1/alerts/${alertId}/acknowledge`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders()
    },
    body: JSON.stringify({ acknowledged_by: acknowledgedBy }),
  });
  
  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to acknowledge alert' }));
    throw new Error(errorData.error || 'Failed to acknowledge alert');
  }
  
  return response.json();
}

export const getNotificationChannels = () => apiRequest<NotificationChannel[]>('/api/v1/notification-channels');

export const createNotificationChannel = (payload: CreateNotificationChannel) =>
  apiRequest<NotificationChannel>('/api/v1/notification-channels', { method: 'POST', body: JSON.stringify(payload) });

export const setNotificationChannelEnabled = (id: string, enabled: boolean) =>
  apiRequest<void>(`/api/v1/notification-channels/${encodeURIComponent(id)}`, { method: 'PUT', body: JSON.stringify({ enabled }) });

export const deleteNotificationChannel = (id: string) =>
  apiRequest<void>(`/api/v1/notification-channels/${encodeURIComponent(id)}`, { method: 'DELETE' });

export const testNotificationChannel = (id: string) =>
  apiRequest<{ delivery_id: number }>(`/api/v1/notification-channels/${encodeURIComponent(id)}/test`, { method: 'POST' });

export const getNotificationDeliveries = () =>
  apiRequest<NotificationDelivery[]>('/api/v1/notification-deliveries?limit=100');

export const getAlertSilences = () => apiRequest<AlertSilence[]>('/api/v1/alert-silences');

export const createAlertSilence = (payload: Omit<AlertSilence, 'id' | 'created_at'>) =>
  apiRequest<AlertSilence>('/api/v1/alert-silences', { method: 'POST', body: JSON.stringify(payload) });

export const deleteAlertSilence = (id: string) =>
  apiRequest<void>(`/api/v1/alert-silences/${encodeURIComponent(id)}`, { method: 'DELETE' });

export const getIncidentTimeline = (agentId?: string) =>
  apiRequest<IncidentEvent[]>(`/api/v1/incidents/timeline?limit=100${agentId ? `&agent_id=${encodeURIComponent(agentId)}` : ''}`);

async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      ...(init.body ? { 'Content-Type': 'application/json' } : {}),
      ...getAuthHeaders(),
      ...init.headers,
    },
  });
  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Request failed' }));
    throw new Error(error.error || `Request failed (${response.status})`);
  }
  if (response.status === 204) return undefined as T;
  return response.json();
}

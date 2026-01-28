import { getAuthHeaders } from './auth-utils';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface AlertRule {
  id: string;
  name: string;
  description?: string;
  metric: string;
  condition: 'greaterthan' | 'lessthan' | 'equals' | 'notequals';
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
  condition: 'greaterthan' | 'lessthan' | 'equals' | 'notequals';
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

export async function acknowledgeAlert(alertId: string, acknowledgedBy: string): Promise<Alert> {  const response = await fetch(`${API_BASE_URL}/api/v1/alerts/${alertId}/acknowledge`, {
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
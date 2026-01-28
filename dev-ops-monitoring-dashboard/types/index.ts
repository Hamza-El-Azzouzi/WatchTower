import React from "react"
export type AgentStatus = 'Healthy' | 'Degraded' | 'Unreachable';

export interface Agent {
  id: string;
  name: string;
  last_seen: string;
  status: AgentStatus;
}

export interface Metric {
  name: string;
  value: number;
  timestamp: string;
}

export interface LatestMetrics {
  agent_id: string;
  metrics: Metric[];
  timestamp?: string;
}

export interface DataPoint {
  timestamp: string;
  value: number;
}

export interface HistoricalMetrics {
  agent_id: string;
  metric: string;
  count: number;
  data_points: DataPoint[];
}

export interface Stats {
  total_agents: number;
  total_metrics: number;
  total_data_points: number;
}

export interface MetricCardProps {
  label: string;
  value: string | number;
  unit?: string;
  percentage?: number;
  color?: 'green' | 'yellow' | 'red' | 'blue';
  icon?: React.ReactNode;
  secondaryValue?: string;
}

export interface Alert {
  id: string;
  agent_id: string;
  rule_id: string;
  rule_name: string;
  severity: string;
  message: string;
  triggered_at: string;
  resolved_at?: string;
  status: 'active' | 'resolved';
}

export interface AlertsResponse {
  active_alerts: Alert[];
  recent_alerts: Alert[];
  total_active: number;
}

export interface DatabaseAgent {
  agent: Agent;
  metrics: LatestMetrics | null;
}

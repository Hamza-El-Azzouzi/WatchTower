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

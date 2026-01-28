import { Agent, LatestMetrics, HistoricalMetrics, Stats } from '@/types';

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export async function getAgents(): Promise<Agent[]> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/v1/agents`, {
      next: { revalidate: 10 }
    });
    if (!response.ok) throw new Error(`Failed to fetch agents: ${response.status}`);
    return response.json();
  } catch (error) {
    console.error('Error fetching agents:', error);
    throw error;
  }
}

export async function getLatestMetrics(agentId: string): Promise<LatestMetrics> {
  try {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/metrics/latest?agent_id=${agentId}`,
      { next: { revalidate: 10 } }
    );
    if (!response.ok) throw new Error(`Failed to fetch latest metrics: ${response.status}`);
    return response.json();
  } catch (error) {
    console.error(`Error fetching latest metrics for ${agentId}:`, error);
    throw error;
  }
}

export async function getHistoricalMetrics(
  agentId: string,
  metric: string,
  limit: number = 100
): Promise<HistoricalMetrics> {
  try {
    const response = await fetch(
      `${API_BASE_URL}/api/v1/metrics?agent_id=${agentId}&metric=${metric}&limit=${limit}`,
      { next: { revalidate: 10 } }
    );
    if (!response.ok) throw new Error(`Failed to fetch historical metrics: ${response.status}`);
    return response.json();
  } catch (error) {
    console.error(`Error fetching historical metrics for ${agentId}:`, error);
    throw error;
  }
}

export async function getStats(): Promise<Stats> {
  try {
    const response = await fetch(`${API_BASE_URL}/api/v1/stats`, {
      next: { revalidate: 10 }
    });
    if (!response.ok) throw new Error(`Failed to fetch stats: ${response.status}`);
    return response.json();
  } catch (error) {
    console.error('Error fetching stats:', error);
    throw error;
  }
}

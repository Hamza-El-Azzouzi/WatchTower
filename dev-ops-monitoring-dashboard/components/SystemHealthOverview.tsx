'use client';

import { useEffect, useState } from 'react';
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  RadialBarChart,
  RadialBar,
} from 'recharts';
import { getLatestMetrics } from '@/lib/api';
import { Agent, LatestMetrics } from '@/types';
import { Activity, Cpu, HardDrive, Network } from 'lucide-react';

interface SystemHealthOverviewProps {
  agents: Agent[];
}

interface MetricData {
  name: string;
  value: number;
  color: string;
}

export default function SystemHealthOverview({ agents }: SystemHealthOverviewProps) {
  const [metricsData, setMetricsData] = useState<Record<string, LatestMetrics>>({});
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchAllMetrics = async () => {
      try {
        const results: Record<string, LatestMetrics> = {};
        await Promise.all(
          agents.map(async (agent) => {
            try {
              const metrics = await getLatestMetrics(agent.id);
              results[agent.id] = metrics;
            } catch (error) {
              console.error(`Failed to fetch metrics for ${agent.id}:`, error);
            }
          })
        );
        setMetricsData(results);
      } catch (error) {
        console.error('Error fetching metrics:', error);
      } finally {
        setLoading(false);
      }
    };

    if (agents.length > 0) {
      fetchAllMetrics();
      const interval = setInterval(fetchAllMetrics, 10000);
      return () => clearInterval(interval);
    }
  }, [agents]);

  const getMetricValue = (agentId: string, metricName: string): number => {
    const metrics = metricsData[agentId];
    if (!metrics) return 0;
    const metric = metrics.metrics.find(m => m.name === metricName);
    return metric ? metric.value : 0;
  };

  // Aggregate data for pie chart - resource distribution
  const getResourceDistribution = (): MetricData[] => {
    if (agents.length === 0) return [];
    
    const totalCpu = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'cpu_usage'), 0);
    const totalMemory = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'memory_usage'), 0);
    const totalDisk = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'disk_usage'), 0);
    
    return [
      { name: 'CPU', value: Number((totalCpu / agents.length).toFixed(1)), color: '#3b82f6' },
      { name: 'Memory', value: Number((totalMemory / agents.length).toFixed(1)), color: '#10b981' },
      { name: 'Disk', value: Number((totalDisk / agents.length).toFixed(1)), color: '#f59e0b' },
    ];
  };

  // Server comparison data for bar chart
  const getServerComparisonData = () => {
    return agents.map(agent => ({
      name: agent.name.length > 15 ? agent.name.substring(0, 15) + '...' : agent.name,
      cpu: Number(getMetricValue(agent.id, 'cpu_usage').toFixed(1)),
      memory: Number(getMetricValue(agent.id, 'memory_usage').toFixed(1)),
      disk: Number(getMetricValue(agent.id, 'disk_usage').toFixed(1)),
    }));
  };

  // Get health score for radial chart
  const getHealthScore = (): number => {
    if (agents.length === 0) return 100;
    
    const avgCpu = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'cpu_usage'), 0) / agents.length;
    const avgMemory = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'memory_usage'), 0) / agents.length;
    const avgDisk = agents.reduce((sum, agent) => sum + getMetricValue(agent.id, 'disk_usage'), 0) / agents.length;
    
    // Calculate health score (100 is perfect, lower is worse)
    const cpuScore = Math.max(0, 100 - avgCpu);
    const memoryScore = Math.max(0, 100 - avgMemory);
    const diskScore = Math.max(0, 100 - avgDisk);
    
    return Number(((cpuScore + memoryScore + diskScore) / 3).toFixed(1));
  };

  const resourceDistribution = getResourceDistribution();
  const serverComparison = getServerComparisonData();
  const healthScore = getHealthScore();

  const radialData = [
    {
      name: 'Health',
      value: healthScore,
      fill: healthScore > 80 ? '#10b981' : healthScore > 60 ? '#f59e0b' : '#ef4444',
    },
  ];

  if (loading || agents.length === 0) {
    return (
      <div className="glass-morphism rounded-xl border border-border p-6">
        <h3 className="text-xl font-bold text-foreground mb-4">System Health Overview</h3>
        <p className="text-muted-foreground">Loading system health data...</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold text-foreground">System Health Overview</h2>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Overall Health Score */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth">
          <div className="flex items-center gap-3 mb-4">
            <Activity className="w-5 h-5 text-accent" />
            <h3 className="text-lg font-semibold text-foreground">Overall Health</h3>
          </div>
          <ResponsiveContainer width="100%" height={200}>
            <RadialBarChart
              cx="50%"
              cy="50%"
              innerRadius="60%"
              outerRadius="90%"
              data={radialData}
              startAngle={180}
              endAngle={0}
            >
              <RadialBar
                background
                dataKey="value"
                cornerRadius={10}
              />
              <text
                x="50%"
                y="50%"
                textAnchor="middle"
                dominantBaseline="middle"
                className="text-3xl font-bold"
                fill={radialData[0].fill}
              >
                {healthScore}%
              </text>
            </RadialBarChart>
          </ResponsiveContainer>
          <p className="text-center text-sm text-muted-foreground mt-2">
            {healthScore > 80 ? 'Excellent' : healthScore > 60 ? 'Good' : 'Needs Attention'}
          </p>
        </div>

        {/* Average Resource Usage */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth">
          <div className="flex items-center gap-3 mb-4">
            <Cpu className="w-5 h-5 text-accent" />
            <h3 className="text-lg font-semibold text-foreground">Resource Distribution</h3>
          </div>
          <ResponsiveContainer width="100%" height={200}>
            <PieChart>
              <Pie
                data={resourceDistribution}
                cx="50%"
                cy="50%"
                labelLine={false}
                label={({ name, value }) => `${name}: ${value}%`}
                outerRadius={80}
                fill="#8884d8"
                dataKey="value"
              >
                {resourceDistribution.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{
                  backgroundColor: 'rgba(26,26,26,0.95)',
                  border: '1px solid rgba(99,102,241,0.3)',
                  borderRadius: '8px',
                  color: '#f5f5f5',
                }}
              />
            </PieChart>
          </ResponsiveContainer>
        </div>

        {/* Quick Stats */}
        <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth">
          <div className="flex items-center gap-3 mb-4">
            <HardDrive className="w-5 h-5 text-accent" />
            <h3 className="text-lg font-semibold text-foreground">Quick Stats</h3>
          </div>
          <div className="space-y-4">
            {resourceDistribution.map((item) => (
              <div key={item.name}>
                <div className="flex justify-between mb-2">
                  <span className="text-sm text-muted-foreground">{item.name}</span>
                  <span className="text-sm font-semibold" style={{ color: item.color }}>
                    {item.value}%
                  </span>
                </div>
                <div className="w-full bg-background/50 rounded-full h-2">
                  <div
                    className="h-2 rounded-full transition-all duration-500"
                    style={{
                      width: `${item.value}%`,
                      backgroundColor: item.color,
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Server Comparison Chart */}
      <div className="glass-morphism rounded-xl border border-border p-6 hover:shadow-lg transition-smooth">
        <div className="flex items-center gap-3 mb-4">
          <Network className="w-5 h-5 text-accent" />
          <h3 className="text-lg font-semibold text-foreground">Server Comparison</h3>
        </div>
        <ResponsiveContainer width="100%" height={300}>
          <BarChart data={serverComparison}>
            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.1)" />
            <XAxis
              dataKey="name"
              stroke="#9ca3af"
              style={{ fontSize: '12px' }}
              angle={-45}
              textAnchor="end"
              height={80}
            />
            <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} />
            <Tooltip
              contentStyle={{
                backgroundColor: 'rgba(26,26,26,0.95)',
                border: '1px solid rgba(99,102,241,0.3)',
                borderRadius: '8px',
                color: '#f5f5f5',
              }}
            />
            <Legend wrapperStyle={{ color: '#9ca3af' }} />
            <Bar dataKey="cpu" fill="#3b82f6" name="CPU %" radius={[8, 8, 0, 0]} />
            <Bar dataKey="memory" fill="#10b981" name="Memory %" radius={[8, 8, 0, 0]} />
            <Bar dataKey="disk" fill="#f59e0b" name="Disk %" radius={[8, 8, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

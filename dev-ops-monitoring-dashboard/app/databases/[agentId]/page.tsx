'use client';

import { useEffect, useState } from 'react';
import { useParams, useRouter } from 'next/navigation';
import Link from 'next/link';
import { ArrowLeft, Database, Activity, TrendingUp, Lock, HardDrive, AlertCircle, Zap, Clock, GitBranch, AlertTriangle, CheckCircle, XCircle } from 'lucide-react';
import { LineChart, Line, AreaChart, Area, BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Legend, PieChart, Pie, Cell, ComposedChart } from 'recharts';
import { getAgents, getLatestMetrics } from '@/lib/api';
import { formatBytes, extractMetric } from '@/lib/metrics-utils';
import { Agent, LatestMetrics } from '@/types';

export default function DatabaseDetailPage() {
  const params = useParams();
  const router = useRouter();
  const agentId = decodeURIComponent(params.agentId as string);

  const [agent, setAgent] = useState<Agent | null>(null);
  const [metrics, setMetrics] = useState<LatestMetrics | null>(null);
  const [historicalData, setHistoricalData] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isInitialLoad = true;

    const fetchData = async () => {
      try {
        if (isInitialLoad) {
          setLoading(true);
        }
        
        const agents = await getAgents();
        const foundAgent = agents.find(a => a.id === agentId);

        if (!foundAgent) {
          setError('Database not found');
          return;
        }

        setAgent(foundAgent);

        const metricsData = await getLatestMetrics(agentId);
        setMetrics(metricsData);

        // Build historical data from metrics
        if (metricsData) {
          const timestamp = new Date().toLocaleTimeString();
          const dataPoint = {
            time: timestamp,
            active: extractMetric(metricsData.metrics, 'db_connections_active'),
            idle: extractMetric(metricsData.metrics, 'db_connections_idle'),
            cacheHit: extractMetric(metricsData.metrics, 'db_cache_hit_ratio'),
            qps: extractMetric(metricsData.metrics, 'db_queries_per_second'),
            locks: extractMetric(metricsData.metrics, 'db_locks_waiting'),
            slowQueries: extractMetric(metricsData.metrics, 'db_slow_queries'),
            committed: extractMetric(metricsData.metrics, 'db_transactions_committed'),
            rolledBack: extractMetric(metricsData.metrics, 'db_transactions_rolled_back'),
            sizeGB: extractMetric(metricsData.metrics, 'db_database_size_bytes') / (1024 ** 3),
          };

          setHistoricalData(prev => {
            const updated = [...prev, dataPoint];
            return updated.slice(-30); // Keep last 30 points
          });
        }

        setError(null);
      } catch {
        setError('Failed to load database details');
      } finally {
        if (isInitialLoad) {
          setLoading(false);
          isInitialLoad = false;
        }
      }
    };

    if (agentId) {
      fetchData(); // Initial load only - consider adding WebSocket for real-time updates
    }
  }, [agentId]);

  if (loading) {
    return (
      <div className="p-8">
        <div className="animate-pulse space-y-6">
          <div className="h-8 bg-gray-700 rounded w-1/3"></div>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
            {[1, 2, 3, 4, 5, 6].map(i => (
              <div key={i} className="h-32 bg-gray-800 rounded-xl"></div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (error || !agent || !metrics) {
    return (
      <div className="p-8">
        <Link href="/databases" className="inline-flex items-center gap-2 text-primary hover:underline mb-6">
          <ArrowLeft className="w-4 h-4" />
          Back to Databases
        </Link>
        <div className="glass-morphism rounded-xl p-12 text-center">
          <AlertCircle className="w-16 h-16 text-red-400 mx-auto mb-4" />
          <h2 className="text-xl font-semibold mb-2">{error || 'Database not found'}</h2>
        </div>
      </div>
    );
  }

  const connectionsActive = extractMetric(metrics.metrics, 'db_connections_active');
  const connectionsIdle = extractMetric(metrics.metrics, 'db_connections_idle');
  const connectionsMax = extractMetric(metrics.metrics, 'db_connections_max');
  const cacheHitRatio = extractMetric(metrics.metrics, 'db_cache_hit_ratio');
  const slowQueries = extractMetric(metrics.metrics, 'db_slow_queries');
  const dbSize = extractMetric(metrics.metrics, 'db_database_size_bytes');
  const locksWaiting = extractMetric(metrics.metrics, 'db_locks_waiting');
  const txCommitted = extractMetric(metrics.metrics, 'db_transactions_committed');
  const txRolledBack = extractMetric(metrics.metrics, 'db_transactions_rolled_back');
  const qps = extractMetric(metrics.metrics, 'db_queries_per_second');

  const totalConnections = connectionsActive + connectionsIdle;
  const connectionUsage = (totalConnections / connectionsMax) * 100;
  
  // Health assessment
  const isHealthy = cacheHitRatio > 90 && connectionUsage < 80 && locksWaiting === 0 && slowQueries < 10;
  const hasWarnings = cacheHitRatio < 90 || connectionUsage > 70 || slowQueries > 0;
  const hasCritical = cacheHitRatio < 80 || connectionUsage > 90 || locksWaiting > 5;

  // Connection pool data for pie chart
  const connectionData = [
    { name: 'Active', value: connectionsActive, color: '#3b82f6' },
    { name: 'Idle', value: connectionsIdle, color: '#10b981' },
    { name: 'Available', value: connectionsMax - totalConnections, color: '#6b7280' },
  ];

  // Transaction success rate
  const totalTx = txCommitted + txRolledBack;
  const successRate = totalTx > 0 ? (txCommitted / totalTx) * 100 : 100;

  return (
    <div className="p-8">
      <Link href="/databases" className="inline-flex items-center gap-2 text-primary hover:underline mb-6">
        <ArrowLeft className="w-4 h-4" />
        Back to Databases
      </Link>

      {/* Header with Health Status */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <Database className="w-8 h-8 text-primary" />
            <h1 className="text-3xl font-bold">{agent.name}</h1>
            {isHealthy && (
              <span className="flex items-center gap-1 px-3 py-1 rounded-full bg-green-500/20 text-green-400 text-sm">
                <CheckCircle className="w-4 h-4" />
                Healthy
              </span>
            )}
            {hasWarnings && !hasCritical && (
              <span className="flex items-center gap-1 px-3 py-1 rounded-full bg-yellow-500/20 text-yellow-400 text-sm">
                <AlertTriangle className="w-4 h-4" />
                Warning
              </span>
            )}
            {hasCritical && (
              <span className="flex items-center gap-1 px-3 py-1 rounded-full bg-red-500/20 text-red-400 text-sm">
                <XCircle className="w-4 h-4" />
                Critical
              </span>
            )}
          </div>
          <p className="text-muted-foreground">Real-time database performance and health monitoring</p>
        </div>
        <div className="text-right text-sm text-muted-foreground">
          <div>Last updated: {metrics.timestamp ? new Date(metrics.timestamp).toLocaleTimeString() : 'N/A'}</div>
          <div className="text-xs mt-1">Refreshes every 10s</div>
        </div>
      </div>

      {/* Key Metrics Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 xl:grid-cols-6 gap-4 mb-6">
        <MetricCard
          icon={<Activity className="w-5 h-5 text-blue-400" />}
          title="Active Connections"
          value={connectionsActive.toString()}
          subtitle={`${totalConnections}/${connectionsMax} total`}
          status={connectionUsage > 90 ? 'critical' : connectionUsage > 70 ? 'warning' : 'good'}
        />
        <MetricCard
          icon={<GitBranch className="w-5 h-5 text-cyan-400" />}
          title="Idle Connections"
          value={connectionsIdle.toString()}
          subtitle={`${((connectionsIdle / connectionsMax) * 100).toFixed(0)}% of pool`}
        />
        <MetricCard
          icon={<TrendingUp className="w-5 h-5 text-green-400" />}
          title="Cache Hit Ratio"
          value={`${cacheHitRatio.toFixed(1)}%`}
          subtitle="Buffer cache perf"
          status={cacheHitRatio < 80 ? 'critical' : cacheHitRatio < 90 ? 'warning' : 'good'}
        />
        <MetricCard
          icon={<Zap className="w-5 h-5 text-yellow-400" />}
          title="Queries/Second"
          value={qps.toFixed(1)}
          subtitle="Query throughput"
        />
        <MetricCard
          icon={<Clock className="w-5 h-5 text-orange-400" />}
          title="Slow Queries"
          value={slowQueries.toString()}
          subtitle="Performance issues"
          status={slowQueries > 10 ? 'critical' : slowQueries > 0 ? 'warning' : 'good'}
        />
        <MetricCard
          icon={<Lock className="w-5 h-5 text-red-400" />}
          title="Waiting Locks"
          value={locksWaiting.toString()}
          subtitle="Blocked queries"
          status={locksWaiting > 5 ? 'critical' : locksWaiting > 0 ? 'warning' : 'good'}
        />
      </div>

      {/* Charts Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-6">
        {/* Connections Over Time - Stacked Area */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <Activity className="w-5 h-5 text-blue-400" />
            <h3 className="text-lg font-semibold">Connection Pool Usage</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <AreaChart data={historicalData}>
                <defs>
                  <linearGradient id="activeGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#3b82f6" stopOpacity={0.8}/>
                    <stop offset="95%" stopColor="#3b82f6" stopOpacity={0.1}/>
                  </linearGradient>
                  <linearGradient id="idleGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#10b981" stopOpacity={0.6}/>
                    <stop offset="95%" stopColor="#10b981" stopOpacity={0.1}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                />
                <Legend />
                <Area type="monotone" dataKey="active" stackId="1" stroke="#3b82f6" fill="url(#activeGradient)" name="Active" />
                <Area type="monotone" dataKey="idle" stackId="1" stroke="#10b981" fill="url(#idleGradient)" name="Idle" />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>

        {/* Cache Hit Ratio Over Time */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-green-400" />
            <h3 className="text-lg font-semibold">Cache Hit Ratio Trend</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <AreaChart data={historicalData}>
                <defs>
                  <linearGradient id="cacheGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#10b981" stopOpacity={0.8}/>
                    <stop offset="95%" stopColor="#10b981" stopOpacity={0.1}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} domain={[0, 100]} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                  formatter={(value: number) => `${value.toFixed(1)}%`}
                />
                <Area type="monotone" dataKey="cacheHit" stroke="#10b981" fill="url(#cacheGradient)" name="Cache Hit %" />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>

        {/* Query Performance */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-5 h-5 text-yellow-400" />
            <h3 className="text-lg font-semibold">Query Performance</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <ComposedChart data={historicalData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis yAxisId="left" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis yAxisId="right" orientation="right" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                />
                <Legend />
                <Line yAxisId="left" type="monotone" dataKey="qps" stroke="#eab308" strokeWidth={2} name="Queries/sec" dot={false} />
                <Bar yAxisId="right" dataKey="slowQueries" fill="#f97316" name="Slow Queries" />
              </ComposedChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>

        {/* Transaction Success Rate */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <CheckCircle className="w-5 h-5 text-emerald-400" />
            <h3 className="text-lg font-semibold">Transaction Health</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <BarChart data={historicalData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                />
                <Legend />
                <Bar dataKey="committed" stackId="a" fill="#10b981" name="Committed" />
                <Bar dataKey="rolledBack" stackId="a" fill="#ef4444" name="Rolled Back" />
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>

        {/* Locks and Database Size */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <Lock className="w-5 h-5 text-red-400" />
            <h3 className="text-lg font-semibold">Lock Contention</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <AreaChart data={historicalData}>
                <defs>
                  <linearGradient id="locksGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#ef4444" stopOpacity={0.8}/>
                    <stop offset="95%" stopColor="#ef4444" stopOpacity={0.1}/>
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                />
                <Area type="monotone" dataKey="locks" stroke="#ef4444" fill="url(#locksGradient)" name="Waiting Locks" />
              </AreaChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>

        {/* Database Size Growth */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <HardDrive className="w-5 h-5 text-purple-400" />
            <h3 className="text-lg font-semibold">Database Size Growth</h3>
          </div>
          {historicalData.length > 0 ? (
            <ResponsiveContainer width="100%" height={280}>
              <LineChart data={historicalData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
                <XAxis dataKey="time" stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <YAxis stroke="#9ca3af" style={{ fontSize: '12px' }} />
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                  labelStyle={{ color: '#f3f4f6' }}
                  formatter={(value: number) => `${value.toFixed(2)} GB`}
                />
                <Line type="monotone" dataKey="sizeGB" stroke="#a855f7" strokeWidth={2} name="Size (GB)" dot={{ fill: '#a855f7', r: 3 }} />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-64 flex items-center justify-center text-muted-foreground">
              Collecting data...
            </div>
          )}
        </div>
      </div>

      {/* Bottom Stats Row */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {/* Connection Pool Breakdown */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <Database className="w-5 h-5 text-blue-400" />
            <h3 className="text-lg font-semibold">Connection Pool</h3>
          </div>
          <div className="flex items-center justify-center">
            <ResponsiveContainer width="100%" height={200}>
              <PieChart>
                <Pie
                  data={connectionData}
                  cx="50%"
                  cy="50%"
                  innerRadius={60}
                  outerRadius={80}
                  paddingAngle={2}
                  dataKey="value"
                >
                  {connectionData.map((entry, index) => (
                    <Cell key={`cell-${index}`} fill={entry.color} />
                  ))}
                </Pie>
                <Tooltip 
                  contentStyle={{ backgroundColor: '#1f2937', border: '1px solid #374151', borderRadius: '8px' }}
                />
              </PieChart>
            </ResponsiveContainer>
          </div>
          <div className="grid grid-cols-3 gap-2 text-center text-sm">
            {connectionData.map((item, i) => (
              <div key={i}>
                <div className="font-semibold" style={{ color: item.color }}>{item.value}</div>
                <div className="text-xs text-muted-foreground">{item.name}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Transaction Statistics */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <CheckCircle className="w-5 h-5 text-emerald-400" />
            <h3 className="text-lg font-semibold">Transaction Stats</h3>
          </div>
          <div className="space-y-4">
            <div>
              <div className="flex justify-between mb-2">
                <span className="text-sm text-muted-foreground">Success Rate</span>
                <span className="text-sm font-semibold text-green-400">{successRate.toFixed(1)}%</span>
              </div>
              <div className="h-2 bg-gray-700 rounded-full overflow-hidden">
                <div 
                  className="h-full bg-gradient-to-r from-green-500 to-emerald-400 transition-all duration-500"
                  style={{ width: `${successRate}%` }}
                />
              </div>
            </div>
            <div className="grid grid-cols-2 gap-4 pt-2">
              <div className="text-center p-3 bg-green-500/10 rounded-lg">
                <div className="text-2xl font-bold text-green-400">{txCommitted}</div>
                <div className="text-xs text-muted-foreground">Committed</div>
              </div>
              <div className="text-center p-3 bg-red-500/10 rounded-lg">
                <div className="text-2xl font-bold text-red-400">{txRolledBack}</div>
                <div className="text-xs text-muted-foreground">Rolled Back</div>
              </div>
            </div>
          </div>
        </div>

        {/* Database Info */}
        <div className="glass-morphism rounded-xl p-6">
          <div className="flex items-center gap-2 mb-4">
            <HardDrive className="w-5 h-5 text-purple-400" />
            <h3 className="text-lg font-semibold">Database Info</h3>
          </div>
          <div className="space-y-3">
            <div className="flex justify-between py-2 border-b border-gray-700">
              <span className="text-sm text-muted-foreground">Size</span>
              <span className="text-sm font-semibold">{formatBytes(dbSize)}</span>
            </div>
            <div className="flex justify-between py-2 border-b border-gray-700">
              <span className="text-sm text-muted-foreground">Max Connections</span>
              <span className="text-sm font-semibold">{connectionsMax}</span>
            </div>
            <div className="flex justify-between py-2 border-b border-gray-700">
              <span className="text-sm text-muted-foreground">Queries/Sec</span>
              <span className="text-sm font-semibold">{qps.toFixed(2)}</span>
            </div>
            <div className="flex justify-between py-2">
              <span className="text-sm text-muted-foreground">Cache Hit Ratio</span>
              <span className={`text-sm font-semibold ${cacheHitRatio > 90 ? 'text-green-400' : 'text-yellow-400'}`}>
                {cacheHitRatio.toFixed(2)}%
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

interface MetricCardProps {
  icon: React.ReactNode;
  title: string;
  value: string;
  subtitle: string;
  status?: 'good' | 'warning' | 'critical';
}

function MetricCard({ icon, title, value, subtitle, status = 'good' }: MetricCardProps) {
  const statusColors = {
    good: 'border-green-500/30 bg-green-500/5',
    warning: 'border-yellow-500/30 bg-yellow-500/5',
    critical: 'border-red-500/30 bg-red-500/5',
  };

  return (
    <div className={`glass-morphism rounded-xl p-4 border ${statusColors[status]}`}>
      <div className="flex items-center gap-2 mb-2">
        {icon}
        <span className="text-sm text-muted-foreground">{title}</span>
      </div>
      <div className="text-2xl font-bold mb-1">{value}</div>
      <div className="text-xs text-muted-foreground">{subtitle}</div>
    </div>
  );
}

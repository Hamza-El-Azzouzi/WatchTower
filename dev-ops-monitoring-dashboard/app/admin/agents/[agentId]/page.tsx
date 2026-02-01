'use client'

import { useState, useEffect } from 'react'
import { useParams, useRouter } from 'next/navigation'
import Link from 'next/link'
import { Shield, Server, Database, Activity, ArrowLeft, Clock, CheckCircle, XCircle, AlertTriangle, Trash2, RefreshCw, Key, Settings } from 'lucide-react'
import { getAgents, getLatestMetrics } from '@/lib/api'
import { listApiKeys, type ApiKey } from '@/lib/api-keys-api'
import { LatestMetrics } from '@/types'
import { extractMetric, formatBytes } from '@/lib/metrics-utils'

interface Agent {
  id: string
  agent_id: string
  name: string
  agent_type: string
  status: string
  last_seen: string
  api_key_id?: number
}

export default function AdminAgentDetailPage() {
  const params = useParams()
  const router = useRouter()
  const agentId = decodeURIComponent(params.agentId as string)
  
  const [agent, setAgent] = useState<Agent | null>(null)
  const [metrics, setMetrics] = useState<LatestMetrics | null>(null)
  const [keys, setKeys] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    loadData()
  }, [agentId])

  const loadData = async () => {
    try {
      setLoading(true)
      const [agentsData, keysData] = await Promise.all([
        getAgents(),
        listApiKeys()
      ])
      
      const foundAgent = agentsData.find((a: Agent) => a.id === agentId)
      if (!foundAgent) {
        setError('Agent not found')
        return
      }
      
      setAgent(foundAgent)
      setKeys(keysData)
      
      try {
        const metricsData = await getLatestMetrics(agentId)
        setMetrics(metricsData)
      } catch {
        // Metrics may not be available
      }
      
      setError(null)
    } catch (err) {
      console.error('Failed to load agent:', err)
      setError('Failed to load agent details')
    } finally {
      setLoading(false)
    }
  }

  const getApiKeyName = (apiKeyId?: number) => {
    if (!apiKeyId) return 'Unknown'
    const key = keys.find(k => k.id === apiKeyId)
    return key?.name || 'Unknown'
  }

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'Healthy':
        return 'bg-green-500/20 text-green-500'
      case 'Degraded':
        return 'bg-yellow-500/20 text-yellow-500'
      case 'Unreachable':
        return 'bg-red-500/20 text-red-500'
      default:
        return 'bg-gray-500/20 text-gray-500'
    }
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Healthy':
        return <CheckCircle className="w-5 h-5 text-green-500" />
      case 'Degraded':
        return <AlertTriangle className="w-5 h-5 text-yellow-500" />
      case 'Unreachable':
        return <XCircle className="w-5 h-5 text-red-500" />
      default:
        return <Activity className="w-5 h-5 text-gray-500" />
    }
  }

  const formatLastSeen = (timestamp: string) => {
    return new Date(timestamp).toLocaleString()
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-background p-8 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
      </div>
    )
  }

  if (error || !agent) {
    return (
      <div className="min-h-screen bg-background p-8">
        <div className="max-w-4xl mx-auto">
          <Link
            href="/admin/agents"
            className="inline-flex items-center gap-2 text-muted-foreground hover:text-foreground mb-6"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Agents
          </Link>
          <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-6 text-center">
            <XCircle className="w-12 h-12 text-red-500 mx-auto mb-4" />
            <h2 className="text-xl font-bold text-foreground mb-2">Agent Not Found</h2>
            <p className="text-muted-foreground">{error || 'The requested agent could not be found.'}</p>
          </div>
        </div>
      </div>
    )
  }

  // Extract metrics
  const cpuPercent = metrics ? extractMetric(metrics.metrics, 'cpu_usage') : 0
  const memoryPercent = metrics ? extractMetric(metrics.metrics, 'memory_usage') : 0
  const diskUsed = metrics ? extractMetric(metrics.metrics, 'disk_used_bytes') : 0
  const diskTotal = metrics ? extractMetric(metrics.metrics, 'disk_total_bytes') : 0

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="mb-8">
          <Link
            href="/admin/agents"
            className="inline-flex items-center gap-2 text-muted-foreground hover:text-foreground mb-4"
          >
            <ArrowLeft className="w-4 h-4" />
            Back to Agents
          </Link>
          
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-4">
              <div className="p-3 rounded-lg bg-primary/10">
                {agent.agent_type === 'server' ? (
                  <Server className="w-8 h-8 text-primary" />
                ) : (
                  <Database className="w-8 h-8 text-primary" />
                )}
              </div>
              <div>
                <h1 className="text-3xl font-bold text-foreground">{agent.name}</h1>
                <p className="text-muted-foreground">{agent.agent_id}</p>
              </div>
            </div>
            
            <div className="flex items-center gap-3">
              <span className={`inline-flex items-center gap-2 px-4 py-2 rounded-full text-sm font-medium ${getStatusColor(agent.status)}`}>
                {getStatusIcon(agent.status)}
                {agent.status}
              </span>
              <button
                onClick={loadData}
                className="px-4 py-2 bg-sidebar-accent/20 text-foreground rounded-lg hover:bg-sidebar-accent/30 transition-colors flex items-center gap-2"
              >
                <RefreshCw className="w-4 h-4" />
                Refresh
              </button>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* Agent Info */}
          <div className="lg:col-span-1 space-y-6">
            <div className="bg-card border border-card-border rounded-lg p-6">
              <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
                <Settings className="w-5 h-5 text-primary" />
                Agent Information
              </h2>
              
              <div className="space-y-4">
                <div>
                  <p className="text-sm text-muted-foreground">Agent ID</p>
                  <p className="text-sm font-mono text-foreground break-all">{agent.agent_id}</p>
                </div>
                
                <div>
                  <p className="text-sm text-muted-foreground">Type</p>
                  <p className="text-sm text-foreground capitalize flex items-center gap-2">
                    {agent.agent_type === 'server' ? (
                      <Server className="w-4 h-4 text-purple-500" />
                    ) : (
                      <Database className="w-4 h-4 text-cyan-500" />
                    )}
                    {agent.agent_type}
                  </p>
                </div>
                
                <div>
                  <p className="text-sm text-muted-foreground">API Key</p>
                  <p className="text-sm text-foreground flex items-center gap-2">
                    <Key className="w-4 h-4 text-blue-500" />
                    {getApiKeyName(agent.api_key_id)}
                  </p>
                </div>
                
                <div>
                  <p className="text-sm text-muted-foreground">Last Seen</p>
                  <p className="text-sm text-foreground flex items-center gap-2">
                    <Clock className="w-4 h-4 text-gray-500" />
                    {formatLastSeen(agent.last_seen)}
                  </p>
                </div>
              </div>
            </div>
            
            {/* Danger Zone */}
            <div className="bg-card border border-red-500/30 rounded-lg p-6">
              <h2 className="text-lg font-semibold text-red-500 mb-4 flex items-center gap-2">
                <AlertTriangle className="w-5 h-5" />
                Danger Zone
              </h2>
              
              <p className="text-sm text-muted-foreground mb-4">
                Removing an agent will delete all its metrics and logs. This action cannot be undone.
              </p>
              
              <button
                className="w-full px-4 py-2 bg-red-500/10 text-red-500 rounded-lg hover:bg-red-500/20 transition-colors flex items-center justify-center gap-2"
              >
                <Trash2 className="w-4 h-4" />
                Remove Agent
              </button>
            </div>
          </div>

          {/* Metrics Overview */}
          <div className="lg:col-span-2">
            <div className="bg-card border border-card-border rounded-lg p-6">
              <h2 className="text-lg font-semibold text-foreground mb-4 flex items-center gap-2">
                <Activity className="w-5 h-5 text-primary" />
                Current Metrics
              </h2>
              
              {!metrics ? (
                <div className="text-center py-8 text-muted-foreground">
                  <Activity className="w-12 h-12 mx-auto mb-4 opacity-50" />
                  <p>No metrics available for this agent</p>
                </div>
              ) : (
                <div className="grid grid-cols-2 gap-4">
                  <div className="p-4 bg-sidebar-accent/10 rounded-lg">
                    <p className="text-sm text-muted-foreground mb-1">CPU Usage</p>
                    <p className="text-2xl font-bold text-foreground">{cpuPercent.toFixed(1)}%</p>
                    <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full transition-all ${
                          cpuPercent < 70 ? 'bg-green-500' : cpuPercent < 85 ? 'bg-yellow-500' : 'bg-red-500'
                        }`}
                        style={{ width: `${Math.min(cpuPercent, 100)}%` }}
                      />
                    </div>
                  </div>
                  
                  <div className="p-4 bg-sidebar-accent/10 rounded-lg">
                    <p className="text-sm text-muted-foreground mb-1">Memory Usage</p>
                    <p className="text-2xl font-bold text-foreground">{memoryPercent.toFixed(1)}%</p>
                    <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full transition-all ${
                          memoryPercent < 70 ? 'bg-green-500' : memoryPercent < 85 ? 'bg-yellow-500' : 'bg-red-500'
                        }`}
                        style={{ width: `${Math.min(memoryPercent, 100)}%` }}
                      />
                    </div>
                  </div>
                  
                  <div className="p-4 bg-sidebar-accent/10 rounded-lg col-span-2">
                    <p className="text-sm text-muted-foreground mb-1">Disk Usage</p>
                    <p className="text-2xl font-bold text-foreground">
                      {formatBytes(diskUsed)} / {formatBytes(diskTotal)}
                    </p>
                    <div className="mt-2 h-2 bg-background/30 rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full transition-all bg-blue-500`}
                        style={{ width: `${diskTotal > 0 ? (diskUsed / diskTotal) * 100 : 0}%` }}
                      />
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

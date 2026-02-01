'use client'

import { useState, useEffect, Suspense } from 'react'
import { useSearchParams } from 'next/navigation'
import { Shield, Server, Database, Activity, Settings, RefreshCw, Clock, CheckCircle, XCircle, AlertTriangle } from 'lucide-react'
import { getAgents } from '@/lib/api'
import { listApiKeys, type ApiKey } from '@/lib/api-keys-api'
import Link from 'next/link'

interface Agent {
  id: string
  agent_id: string
  name: string
  agent_type: string
  status: string
  last_seen: string
  api_key_id?: number
}

function AdminAgentsContent() {
  const searchParams = useSearchParams()
  const typeFilter = searchParams.get('type')
  
  const [agents, setAgents] = useState<Agent[]>([])
  const [keys, setKeys] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [filter, setFilter] = useState<string>(typeFilter || 'all')

  useEffect(() => {
    loadData()
  }, [])

  useEffect(() => {
    if (typeFilter) {
      setFilter(typeFilter)
    }
  }, [typeFilter])

  const loadData = async () => {
    try {
      setLoading(true)
      const [agentsData, keysData] = await Promise.all([
        getAgents(),
        listApiKeys()
      ])
      setAgents(agentsData)
      setKeys(keysData)
    } catch (err) {
      console.error('Failed to load agents:', err)
    } finally {
      setLoading(false)
    }
  }

  const filteredAgents = agents.filter((agent: Agent) => {
    if (filter === 'all') return true
    if (filter === 'server') return agent.agent_type === 'server'
    if (filter === 'database') return agent.agent_type === 'database'
    if (filter === 'healthy') return agent.status === 'Healthy'
    if (filter === 'degraded') return agent.status === 'Degraded'
    if (filter === 'unreachable') return agent.status === 'Unreachable'
    return true
  })

  const getApiKeyName = (apiKeyId?: number) => {
    if (!apiKeyId) return 'Unknown'
    const key = keys.find((k: ApiKey) => k.id === apiKeyId)
    return key?.name || 'Unknown'
  }

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'Healthy':
        return <CheckCircle className="w-4 h-4 text-green-500" />
      case 'Degraded':
        return <AlertTriangle className="w-4 h-4 text-yellow-500" />
      case 'Unreachable':
        return <XCircle className="w-4 h-4 text-red-500" />
      default:
        return <Activity className="w-4 h-4 text-gray-500" />
    }
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

  const formatLastSeen = (timestamp: string) => {
    const date = new Date(timestamp)
    const now = new Date()
    const diffMs = now.getTime() - date.getTime()
    const diffSec = Math.floor(diffMs / 1000)
    
    if (diffSec < 60) return `${diffSec}s ago`
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`
    return `${Math.floor(diffSec / 86400)}d ago`
  }

  const serverCount = agents.filter((a: Agent) => a.agent_type === 'server').length
  const dbCount = agents.filter((a: Agent) => a.agent_type === 'database').length
  const healthyCount = agents.filter((a: Agent) => a.status === 'Healthy').length
  const degradedCount = agents.filter((a: Agent) => a.status === 'Degraded').length
  const unreachableCount = agents.filter((a: Agent) => a.status === 'Unreachable').length

  if (loading) {
    return (
      <div className="min-h-screen bg-background p-8 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary"></div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-foreground flex items-center gap-3">
              <Shield className="w-8 h-8 text-primary" />
              Agent Management
            </h1>
            <p className="text-muted-foreground mt-2">
              View and manage all registered agents
            </p>
          </div>
          <button
            onClick={loadData}
            className="px-4 py-2 bg-sidebar-accent/20 text-foreground rounded-lg hover:bg-sidebar-accent/30 transition-colors flex items-center gap-2"
          >
            <RefreshCw className="w-4 h-4" />
            Refresh
          </button>
        </div>

        {/* Stats Cards */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 mb-8">
          <button
            onClick={() => setFilter('all')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'all' 
                ? 'bg-primary/20 border-primary' 
                : 'bg-card border-card-border hover:border-primary/50'
            }`}
          >
            <p className="text-sm text-muted-foreground">Total Agents</p>
            <p className="text-2xl font-bold text-foreground">{agents.length}</p>
          </button>
          
          <button
            onClick={() => setFilter('server')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'server' 
                ? 'bg-purple-500/20 border-purple-500' 
                : 'bg-card border-card-border hover:border-purple-500/50'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <Server className="w-4 h-4 text-purple-500" />
              <p className="text-sm text-muted-foreground">Server</p>
            </div>
            <p className="text-2xl font-bold text-foreground">{serverCount}</p>
          </button>
          
          <button
            onClick={() => setFilter('database')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'database' 
                ? 'bg-cyan-500/20 border-cyan-500' 
                : 'bg-card border-card-border hover:border-cyan-500/50'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <Database className="w-4 h-4 text-cyan-500" />
              <p className="text-sm text-muted-foreground">Database</p>
            </div>
            <p className="text-2xl font-bold text-foreground">{dbCount}</p>
          </button>
          
          <button
            onClick={() => setFilter('healthy')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'healthy' 
                ? 'bg-green-500/20 border-green-500' 
                : 'bg-card border-card-border hover:border-green-500/50'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <CheckCircle className="w-4 h-4 text-green-500" />
              <p className="text-sm text-muted-foreground">Healthy</p>
            </div>
            <p className="text-2xl font-bold text-foreground">{healthyCount}</p>
          </button>
          
          <button
            onClick={() => setFilter('degraded')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'degraded' 
                ? 'bg-yellow-500/20 border-yellow-500' 
                : 'bg-card border-card-border hover:border-yellow-500/50'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <AlertTriangle className="w-4 h-4 text-yellow-500" />
              <p className="text-sm text-muted-foreground">Degraded</p>
            </div>
            <p className="text-2xl font-bold text-foreground">{degradedCount}</p>
          </button>
          
          <button
            onClick={() => setFilter('unreachable')}
            className={`p-4 rounded-lg border transition-colors ${
              filter === 'unreachable' 
                ? 'bg-red-500/20 border-red-500' 
                : 'bg-card border-card-border hover:border-red-500/50'
            }`}
          >
            <div className="flex items-center gap-2 mb-1">
              <XCircle className="w-4 h-4 text-red-500" />
              <p className="text-sm text-muted-foreground">Unreachable</p>
            </div>
            <p className="text-2xl font-bold text-foreground">{unreachableCount}</p>
          </button>
        </div>

        {/* Agents Table */}
        <div className="bg-card border border-card-border rounded-lg overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-card-border bg-sidebar-accent/10">
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Agent Name</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Type</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Status</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">API Key</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Last Seen</th>
                  <th className="text-left py-4 px-6 text-sm font-medium text-muted-foreground">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredAgents.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="py-12 text-center text-muted-foreground">
                      No agents found matching the filter
                    </td>
                  </tr>
                ) : (
                  filteredAgents.map((agent) => (
                    <tr key={agent.id} className="border-b border-card-border hover:bg-sidebar-accent/5">
                      <td className="py-4 px-6">
                        <div>
                          <p className="text-sm font-medium text-foreground">{agent.name}</p>
                          <p className="text-xs text-muted-foreground">{agent.agent_id}</p>
                        </div>
                      </td>
                      <td className="py-4 px-6">
                        <div className="flex items-center gap-2">
                          {agent.agent_type === 'server' ? (
                            <Server className="w-4 h-4 text-purple-500" />
                          ) : (
                            <Database className="w-4 h-4 text-cyan-500" />
                          )}
                          <span className="text-sm capitalize">{agent.agent_type}</span>
                        </div>
                      </td>
                      <td className="py-4 px-6">
                        <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${getStatusColor(agent.status)}`}>
                          {getStatusIcon(agent.status)}
                          {agent.status}
                        </span>
                      </td>
                      <td className="py-4 px-6">
                        <span className="text-sm text-muted-foreground">
                          {getApiKeyName(agent.api_key_id)}
                        </span>
                      </td>
                      <td className="py-4 px-6">
                        <div className="flex items-center gap-2 text-sm text-muted-foreground">
                          <Clock className="w-4 h-4" />
                          {formatLastSeen(agent.last_seen)}
                        </div>
                      </td>
                      <td className="py-4 px-6">
                        <Link
                          href={`/admin/agents/${encodeURIComponent(agent.id)}`}
                          className="text-sm text-primary hover:underline flex items-center gap-1"
                        >
                          <Settings className="w-4 h-4" />
                          Manage
                        </Link>
                      </td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  )
}

export default function AdminAgentsPage() {
  return (
    <Suspense fallback={
      <div className="min-h-screen bg-gradient-to-b from-background to-secondary/20">
        <div className="container mx-auto px-4 py-8">
          <div className="mb-8">
            <h1 className="text-3xl font-bold mb-2">Agent Management</h1>
            <p className="text-muted-foreground">Loading agents...</p>
          </div>
        </div>
      </div>
    }>
      <AdminAgentsContent />
    </Suspense>
  )
}

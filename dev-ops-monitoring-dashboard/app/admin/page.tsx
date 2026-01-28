'use client'

import { useState, useEffect } from 'react'
import { Shield, Key, Activity, Server, Database, TrendingUp, Users, AlertTriangle } from 'lucide-react'
import { listApiKeys, type ApiKey } from '@/lib/api-keys-api'
import { getAgents, getStats } from '@/lib/api'
import { getAlerts } from '@/lib/alerts-api'
import Link from 'next/link'

export default function AdminDashboard() {
  const [keys, setKeys] = useState<ApiKey[]>([])
  const [agents, setAgents] = useState<any[]>([])
  const [stats, setStats] = useState<any>(null)
  const [alerts, setAlerts] = useState<any>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadData()
  }, [])

  const loadData = async () => {
    try {
      setLoading(true)
      const [keysData, agentsData, statsData, alertsData] = await Promise.all([
        listApiKeys(),
        getAgents(),
        getStats(),
        getAlerts()
      ])
      setKeys(keysData)
      setAgents(agentsData)
      setStats(statsData)
      setAlerts(alertsData)
    } catch (err) {
      console.error('Failed to load admin data:', err)
    } finally {
      setLoading(false)
    }
  }

  const activeKeys = keys.filter(k => !k.revoked && (!k.expires_at || new Date(k.expires_at) > new Date()))
  const totalAgents = agents.length
  const serverAgents = agents.filter(a => a.agent_type === 'server').length
  const dbAgents = agents.filter(a => a.agent_type === 'database').length
  const activeAlerts = alerts?.active_alerts?.filter((a: any) => a.state === 'firing').length || 0

  const statsCards = [
    {
      title: 'Active API Keys',
      value: activeKeys.length,
      total: keys.length,
      icon: Key,
      color: 'text-blue-500',
      bgColor: 'bg-blue-500/10',
      link: '/admin/api-keys'
    },
    {
      title: 'Total Agents',
      value: totalAgents,
      icon: Activity,
      color: 'text-green-500',
      bgColor: 'bg-green-500/10',
      link: '/'
    },
    {
      title: 'Server Agents',
      value: serverAgents,
      icon: Server,
      color: 'text-purple-500',
      bgColor: 'bg-purple-500/10',
      link: '/'
    },
    {
      title: 'Database Agents',
      value: dbAgents,
      icon: Database,
      color: 'text-cyan-500',
      bgColor: 'bg-cyan-500/10',
      link: '/databases'
    },
    {
      title: 'Active Alerts',
      value: activeAlerts,
      icon: AlertTriangle,
      color: 'text-red-500',
      bgColor: 'bg-red-500/10',
      link: '/alerts'
    }
  ]

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
        <div className="mb-8">
          <div className="flex items-center gap-3 mb-2">
            <Shield className="w-8 h-8 text-primary" />
            <h1 className="text-3xl font-bold text-foreground">Admin Dashboard</h1>
          </div>
          <p className="text-muted-foreground">
            Monitor and manage your DevOps monitoring system
          </p>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-4 mb-8">
          {statsCards.map((stat, idx) => {
            const Icon = stat.icon
            return (
              <Link
                key={idx}
                href={stat.link}
                className="bg-card border border-card-border rounded-lg p-6 hover:border-primary/50 transition-colors"
              >
                <div className="flex items-center justify-between mb-4">
                  <div className={`p-3 rounded-lg ${stat.bgColor}`}>
                    <Icon className={`w-6 h-6 ${stat.color}`} />
                  </div>
                </div>
                <div>
                  <p className="text-sm text-muted-foreground mb-1">{stat.title}</p>
                  <p className="text-3xl font-bold text-foreground">
                    {stat.value}
                    {stat.total && (
                      <span className="text-lg text-muted-foreground ml-2">/ {stat.total}</span>
                    )}
                  </p>
                </div>
              </Link>
            )
          })}
        </div>

        {/* API Keys Overview */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 mb-8">
          {/* Recent API Keys */}
          <div className="bg-card border border-card-border rounded-lg p-6">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-xl font-semibold text-foreground flex items-center gap-2">
                <Key className="w-5 h-5 text-primary" />
                Recent API Keys
              </h2>
              <Link
                href="/admin/api-keys"
                className="text-sm text-primary hover:underline"
              >
                View All
              </Link>
            </div>
            <div className="space-y-3">
              {keys.slice(0, 5).map((key) => {
                const isActive = !key.revoked && (!key.expires_at || new Date(key.expires_at) > new Date())
                return (
                  <div key={key.id} className="flex items-center justify-between p-3 bg-sidebar-accent/10 rounded-lg">
                    <div className="flex-1">
                      <p className="text-sm font-medium text-foreground">{key.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {key.used_by_agents?.length || 0} agents
                      </p>
                    </div>
                    <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                      isActive ? 'bg-green-500/20 text-green-500' : 'bg-red-500/20 text-red-500'
                    }`}>
                      {isActive ? 'Active' : 'Inactive'}
                    </span>
                  </div>
                )
              })}
            </div>
          </div>

          {/* Agent Distribution */}
          <div className="bg-card border border-card-border rounded-lg p-6">
            <h2 className="text-xl font-semibold text-foreground flex items-center gap-2 mb-4">
              <Activity className="w-5 h-5 text-primary" />
              Agent Distribution
            </h2>
            <div className="space-y-4">
              <div>
                <div className="flex justify-between text-sm mb-2">
                  <span className="text-muted-foreground">Server Agents</span>
                  <span className="text-foreground font-medium">{serverAgents} / {totalAgents}</span>
                </div>
                <div className="w-full bg-sidebar-accent/20 rounded-full h-2">
                  <div
                    className="bg-purple-500 h-2 rounded-full transition-all"
                    style={{ width: `${totalAgents > 0 ? (serverAgents / totalAgents) * 100 : 0}%` }}
                  />
                </div>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-2">
                  <span className="text-muted-foreground">Database Agents</span>
                  <span className="text-foreground font-medium">{dbAgents} / {totalAgents}</span>
                </div>
                <div className="w-full bg-sidebar-accent/20 rounded-full h-2">
                  <div
                    className="bg-cyan-500 h-2 rounded-full transition-all"
                    style={{ width: `${totalAgents > 0 ? (dbAgents / totalAgents) * 100 : 0}%` }}
                  />
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* API Key Usage Details */}
        <div className="bg-card border border-card-border rounded-lg p-6">
          <h2 className="text-xl font-semibold text-foreground flex items-center gap-2 mb-6">
            <TrendingUp className="w-5 h-5 text-primary" />
            API Key Usage
          </h2>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-card-border">
                  <th className="text-left py-3 px-4 text-sm font-medium text-muted-foreground">Key Name</th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-muted-foreground">Agents</th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-muted-foreground">Limit</th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-muted-foreground">Last Used</th>
                  <th className="text-left py-3 px-4 text-sm font-medium text-muted-foreground">Status</th>
                </tr>
              </thead>
              <tbody>
                {keys.map((key) => {
                  const isActive = !key.revoked && (!key.expires_at || new Date(key.expires_at) > new Date())
                  const agentCount = key.used_by_agents?.length || 0
                  const limitReached = key.max_agents && agentCount >= key.max_agents
                  
                  return (
                    <tr key={key.id} className="border-b border-card-border hover:bg-sidebar-accent/5">
                      <td className="py-3 px-4 text-sm text-foreground font-medium">{key.name}</td>
                      <td className="py-3 px-4">
                        <span className={`text-sm ${limitReached ? 'text-orange-500 font-semibold' : 'text-foreground'}`}>
                          {agentCount}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-sm text-muted-foreground">
                        {key.max_agents ? key.max_agents : 'Unlimited'}
                      </td>
                      <td className="py-3 px-4 text-sm text-muted-foreground">
                        {key.last_used_at ? new Date(key.last_used_at).toLocaleDateString() : 'Never'}
                      </td>
                      <td className="py-3 px-4">
                        <span className={`px-2 py-1 rounded-full text-xs font-medium ${
                          isActive ? 'bg-green-500/20 text-green-500' : 'bg-red-500/20 text-red-500'
                        }`}>
                          {isActive ? 'Active' : 'Inactive'}
                        </span>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  )
}

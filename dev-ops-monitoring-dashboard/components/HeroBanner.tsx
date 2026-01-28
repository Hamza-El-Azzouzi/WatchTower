'use client'

import { useEffect, useState } from 'react'
import { Activity, TrendingUp, Zap } from 'lucide-react'
import { getAgents, getLatestMetrics } from '@/lib/api'
import { Agent } from '@/types'

export default function HeroBanner() {
  const [metrics, setMetrics] = useState({
    cpu: 0,
    memory: 0,
    uptime: 0,
    cpuStatus: 'Loading...',
    memoryStatus: 'Loading...',
  })

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        const agents = await getAgents()
        if (agents.length === 0) return

        // Get metrics from the first agent (or aggregate from all)
        const firstAgent = agents[0]
        const latestMetrics = await getLatestMetrics(firstAgent.id)

        const cpuMetric = latestMetrics.metrics.find(m => m.name === 'cpu_usage')
        const memoryMetric = latestMetrics.metrics.find(m => m.name === 'memory_usage')

        const cpuValue = cpuMetric?.value || 0
        const memoryValue = memoryMetric?.value || 0

        // Calculate uptime based on agent status (simplified)
        const healthyAgents = agents.filter(a => a.status === 'Healthy').length
        const uptimePercent = agents.length > 0 ? (healthyAgents / agents.length) * 100 : 0

        setMetrics({
          cpu: cpuValue,
          memory: memoryValue,
          uptime: uptimePercent,
          cpuStatus: cpuValue < 70 ? 'Normal' : cpuValue < 90 ? 'Warning' : 'Critical',
          memoryStatus: memoryValue < 75 ? 'Stable' : memoryValue < 90 ? 'Warning' : 'Critical',
        })
      } catch (error) {
        console.error('Failed to fetch metrics for hero banner:', error)
      }
    }

    fetchMetrics()
    const interval = setInterval(fetchMetrics, 10000) // Update every 10 seconds
    return () => clearInterval(interval)
  }, [])
  return (
    <div className="relative overflow-hidden bg-gradient-to-br from-primary/10 via-background to-accent/5 rounded-2xl border border-border/50 p-8 mb-8 animate-slide-up">
      {/* Animated background elements */}
      <div className="absolute inset-0 overflow-hidden">
        <div className="absolute -top-40 -right-40 w-80 h-80 bg-primary/20 rounded-full blur-3xl opacity-20 animate-float" />
        <div className="absolute -bottom-20 -left-40 w-80 h-80 bg-accent/20 rounded-full blur-3xl opacity-20 animate-float" style={{ animationDelay: '1s' }} />
      </div>

      {/* Content */}
      <div className="relative z-10">
        <div className="flex items-start justify-between mb-6">
          <div>
            <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-lg bg-primary/20 border border-primary/30 mb-4">
              <Zap className="w-4 h-4 text-primary" />
              <span className="text-sm font-medium text-primary">Real-time Monitoring</span>
            </div>
            <h2 className="text-3xl font-bold text-foreground mb-2">Infrastructure at Your Fingertips</h2>
            <p className="text-muted-foreground text-lg max-w-2xl">
              Monitor all your servers in real-time with beautiful charts, instant alerts, and comprehensive metrics. Stay ahead of issues before they impact your systems.
            </p>
          </div>
          <div className="hidden lg:flex items-center gap-4">
            <div className="w-12 h-12 rounded-lg bg-primary/20 flex items-center justify-center animate-glow">
              <Activity className="w-6 h-6 text-primary" />
            </div>
            <div className="w-12 h-12 rounded-lg bg-accent/20 flex items-center justify-center animate-glow" style={{ animationDelay: '1s' }}>
              <TrendingUp className="w-6 h-6 text-accent" />
            </div>
          </div>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div className="p-4 rounded-lg bg-background/40 border border-border/50 hover:border-primary/30 transition-smooth">
            <div className="text-sm font-medium text-muted-foreground mb-1">CPU Usage</div>
            <div className="text-2xl font-bold text-primary">{metrics.cpu.toFixed(1)}%</div>
            <div className="text-xs text-muted-foreground mt-1">{metrics.cpuStatus}</div>
          </div>
          <div className="p-4 rounded-lg bg-background/40 border border-border/50 hover:border-accent/30 transition-smooth">
            <div className="text-sm font-medium text-muted-foreground mb-1">Memory</div>
            <div className="text-2xl font-bold text-accent">{metrics.memory.toFixed(1)}%</div>
            <div className="text-xs text-muted-foreground mt-1">{metrics.memoryStatus}</div>
          </div>
          <div className="p-4 rounded-lg bg-background/40 border border-border/50 hover:border-emerald-400/30 transition-smooth">
            <div className="text-sm font-medium text-muted-foreground mb-1">Uptime</div>
            <div className="text-2xl font-bold text-emerald-400">{metrics.uptime.toFixed(1)}%</div>
            <div className="text-xs text-muted-foreground mt-1">{metrics.uptime >= 99 ? 'Excellent' : metrics.uptime >= 95 ? 'Good' : 'Degraded'}</div>
          </div>
        </div>
      </div>
    </div>
  )
}

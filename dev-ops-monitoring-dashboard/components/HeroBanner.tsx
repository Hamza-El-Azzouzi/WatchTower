'use client'

import { Activity, TrendingUp, Zap } from 'lucide-react'

export default function HeroBanner() {
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
            <div className="text-2xl font-bold text-primary">45.2%</div>
            <div className="text-xs text-muted-foreground mt-1">Normal</div>
          </div>
          <div className="p-4 rounded-lg bg-background/40 border border-border/50 hover:border-accent/30 transition-smooth">
            <div className="text-sm font-medium text-muted-foreground mb-1">Memory</div>
            <div className="text-2xl font-bold text-accent">62.8%</div>
            <div className="text-xs text-muted-foreground mt-1">Stable</div>
          </div>
          <div className="p-4 rounded-lg bg-background/40 border border-border/50 hover:border-emerald-400/30 transition-smooth">
            <div className="text-sm font-medium text-muted-foreground mb-1">Uptime</div>
            <div className="text-2xl font-bold text-emerald-400">99.9%</div>
            <div className="text-xs text-muted-foreground mt-1">Excellent</div>
          </div>
        </div>
      </div>
    </div>
  )
}

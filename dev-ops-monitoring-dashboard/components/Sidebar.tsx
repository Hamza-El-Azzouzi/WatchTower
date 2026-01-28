'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Activity, Home, Database, Zap, Settings, Bell, HelpCircle, Shield, Key } from 'lucide-react'
import { useEffect, useState } from 'react'
import { getAlerts } from '@/lib/alerts-api'

export function Sidebar() {
  const pathname = usePathname()
  const [alertCount, setAlertCount] = useState(0)
  
  const isActive = (href: string) => pathname === href || pathname.startsWith(href + '/')
  
  useEffect(() => {
    const fetchAlertCount = async () => {
      try {
        const response = await getAlerts()
        const activeCount = response.active_alerts.filter((a: { state: string }) => a.state === 'firing' || a.state === 'pending').length
        setAlertCount(activeCount)
      } catch (error) {
        console.error('Failed to fetch alerts:', error)
      }
    }
    
    fetchAlertCount()
    const interval = setInterval(fetchAlertCount, 10000) // Update every 10 seconds
    
    return () => clearInterval(interval)
  }, [])
  
  const navItems = [
    { icon: Home, label: 'Overview', href: '/', id: 'overview' },
    { icon: Activity, label: 'Servers', href: '/', id: 'servers' },
    { icon: Database, label: 'Databases', href: '/databases', id: 'databases' },
    { icon: Zap, label: 'Performance', href: '/performance', id: 'performance' },
    { icon: Bell, label: 'Alerts', href: '/alerts', id: 'alerts', badge: alertCount },
  ]

  const adminItems = [
    { icon: Key, label: 'API Keys', href: '/admin/api-keys', id: 'api-keys' },
  ]
  
  return (
    <aside className="fixed left-0 top-0 h-screen w-64 bg-sidebar border-r border-sidebar-border p-6 flex flex-col justify-between animate-slide-up">
      {/* Logo */}
      <div className="flex flex-col gap-8">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-gradient-to-br from-primary to-accent flex items-center justify-center animate-glow">
            <Activity className="w-6 h-6 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-bold text-sidebar-foreground">DevOps Pro</h1>
            <p className="text-xs text-muted-foreground">Monitoring</p>
          </div>
        </div>
        
        {/* Navigation */}
        <nav className="flex flex-col gap-2">
          {navItems.map((item) => {
            const Icon = item.icon
            const active = isActive(item.href)
            const hasBadge = item.badge !== undefined && item.badge > 0
            return (
              <Link
                key={item.id}
                href={item.href}
                className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-smooth group relative ${
                  active
                    ? 'bg-sidebar-primary text-sidebar-primary-foreground shadow-lg'
                    : 'text-sidebar-foreground hover:bg-sidebar-accent/20'
                }`}
              >
                <Icon className="w-5 h-5" />
                <span className="text-sm font-medium">{item.label}</span>
                {hasBadge && (
                  <span className="ml-auto min-w-[20px] h-5 px-1.5 flex items-center justify-center rounded-full bg-red-500 text-white text-xs font-bold animate-pulse-soft">
                    {item.badge}
                  </span>
                )}
                {active && !hasBadge && <div className="ml-auto w-2 h-2 rounded-full bg-sidebar-accent animate-pulse-soft" />}
              </Link>
            )
          })}

          {/* Admin Section */}
          <div className="mt-4 pt-4 border-t border-sidebar-border">
            <div className="flex items-center gap-2 px-4 mb-2">
              <Shield className="w-4 h-4 text-muted-foreground" />
              <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">Admin</span>
            </div>
            {adminItems.map((item) => {
              const Icon = item.icon
              const active = isActive(item.href)
              return (
                <Link
                  key={item.id}
                  href={item.href}
                  className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-smooth group relative ${
                    active
                      ? 'bg-sidebar-primary text-sidebar-primary-foreground shadow-lg'
                      : 'text-sidebar-foreground hover:bg-sidebar-accent/20'
                  }`}
                >
                  <Icon className="w-5 h-5" />
                  <span className="text-sm font-medium">{item.label}</span>
                  {active && <div className="ml-auto w-2 h-2 rounded-full bg-sidebar-accent animate-pulse-soft" />}
                </Link>
              )
            })}
          </div>
        </nav>
      </div>
      
      {/* Footer */}
      <div className="flex flex-col gap-2 pt-6 border-t border-sidebar-border">
        <button className="flex items-center gap-3 px-4 py-3 rounded-lg text-sidebar-foreground hover:bg-sidebar-accent/20 transition-smooth group w-full">
          <HelpCircle className="w-5 h-5" />
          <span className="text-sm font-medium">Help</span>
        </button>
        <button className="flex items-center gap-3 px-4 py-3 rounded-lg text-sidebar-foreground hover:bg-sidebar-accent/20 transition-smooth group w-full">
          <Settings className="w-5 h-5" />
          <span className="text-sm font-medium">Settings</span>
        </button>
      </div>
    </aside>
  )
}

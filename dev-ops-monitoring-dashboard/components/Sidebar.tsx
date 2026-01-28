'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Activity, Home, Database, Zap, Settings, Bell, HelpCircle } from 'lucide-react'

export function Sidebar() {
  const pathname = usePathname()
  
  const isActive = (href: string) => pathname === href || pathname.startsWith(href + '/')
  
  const navItems = [
    { icon: Home, label: 'Overview', href: '/' },
    { icon: Activity, label: 'Servers', href: '/' },
    { icon: Database, label: 'Database', href: '#' },
    { icon: Zap, label: 'Performance', href: '#' },
    { icon: Bell, label: 'Alerts', href: '#' },
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
            return (
              <Link
                key={item.href}
                href={item.href}
                className={`flex items-center gap-3 px-4 py-3 rounded-lg transition-smooth group ${
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

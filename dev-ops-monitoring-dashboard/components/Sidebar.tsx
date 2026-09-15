'use client'

import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { Activity, Home, Database, Zap, Bell, Shield, Key, LogOut, Users, Lock, FileText, Menu, X, ChevronRight } from 'lucide-react'
import { useEffect, useState, useCallback } from 'react'
import { getAlerts } from '@/lib/alerts-api'
import { useAlertsWebSocket } from '@/hooks/useWebSocket'
import { WsAlertMessage } from '@/lib/websocket'

export function Sidebar() {
  const pathname = usePathname()
  const router = useRouter()
  const [alertCount, setAlertCount] = useState(0)
  const [userType, setUserType] = useState<string | null>(null)
  const [mobileOpen, setMobileOpen] = useState(false)

  const handleAlertUpdate = useCallback((message: WsAlertMessage) => {
    if (message.state === 'firing' || message.state === 'pending') setAlertCount(count => count + 1)
    if (message.state === 'resolved') setAlertCount(count => Math.max(0, count - 1))
  }, [])

  useAlertsWebSocket(handleAlertUpdate)

  useEffect(() => {
    const type = localStorage.getItem('user_type')
    setUserType(type)
    if (type !== 'admin') {
      getAlerts()
        .then(response => setAlertCount(response.active_alerts.filter((alert: { state: string }) => alert.state === 'firing' || alert.state === 'pending').length))
        .catch(error => console.error('Failed to fetch alerts:', error))
    }
  }, [])

  useEffect(() => setMobileOpen(false), [pathname])

  const monitoringItems = [
    { icon: Home, label: 'Overview', href: '/', id: 'overview' },
    { icon: Activity, label: 'Servers', href: '/#servers', id: 'servers' },
    { icon: Database, label: 'Databases', href: '/databases', id: 'databases' },
    { icon: FileText, label: 'Logs', href: '/logs', id: 'logs' },
    { icon: Zap, label: 'Performance', href: '/performance', id: 'performance' },
    { icon: Bell, label: 'Alerts', href: '/alerts', id: 'alerts', badge: alertCount },
  ]
  const adminItems = [
    { icon: Shield, label: 'Control center', href: '/admin', id: 'admin-overview' },
    { icon: Activity, label: 'Agents', href: '/admin/agents', id: 'admin-agents' },
    { icon: Bell, label: 'Alerts', href: '/admin/alerts', id: 'admin-alerts' },
    { icon: Key, label: 'API keys', href: '/admin/api-keys', id: 'api-keys' },
    { icon: Users, label: 'Administrators', href: '/admin/users', id: 'manage-admins' },
    { icon: Lock, label: 'Security', href: '/admin/change-password', id: 'change-password' },
  ]
  const navItems = userType === 'admin' ? adminItems : monitoringItems

  const isActive = (item: { href: string; id: string }) => {
    if (item.id === 'servers') return false
    if (item.href === '/') return pathname === '/'
    return pathname === item.href || pathname.startsWith(`${item.href}/`)
  }

  const handleLogout = () => {
    localStorage.removeItem('api_key')
    localStorage.removeItem('admin_token')
    localStorage.removeItem('user_type')
    router.push(userType === 'admin' ? '/admin-login' : '/login')
  }

  return (
    <>
      <header className="fixed inset-x-0 top-0 z-50 flex h-16 items-center justify-between border-b border-white/8 bg-[#080d15]/90 px-4 backdrop-blur-xl lg:hidden">
        <Brand compact />
        <button onClick={() => setMobileOpen(true)} className="rounded-xl border border-white/10 bg-white/5 p-2.5 text-foreground" aria-label="Open navigation">
          <Menu className="h-5 w-5" />
        </button>
      </header>

      {mobileOpen && <button className="fixed inset-0 z-50 bg-black/65 backdrop-blur-sm lg:hidden" onClick={() => setMobileOpen(false)} aria-label="Close navigation overlay" />}

      <aside className={`fixed inset-y-0 left-0 z-[60] flex w-72 flex-col border-r border-white/8 bg-[#080d15]/96 px-4 py-5 backdrop-blur-2xl transition-transform duration-300 lg:translate-x-0 ${mobileOpen ? 'translate-x-0' : '-translate-x-full'}`}>
        <div className="flex items-center justify-between px-2">
          <Brand />
          <button onClick={() => setMobileOpen(false)} className="rounded-lg p-2 text-muted-foreground hover:bg-white/5 hover:text-foreground lg:hidden" aria-label="Close navigation">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="mt-9 px-3"><p className="eyebrow">{userType === 'admin' ? 'Administration' : 'Workspace'}</p></div>
        <nav className="mt-3 flex flex-1 flex-col gap-1" aria-label="Primary navigation">
          {navItems.map(item => {
            const Icon = item.icon
            const active = isActive(item)
            const badge = 'badge' in item && typeof item.badge === 'number' ? item.badge : 0
            return (
              <Link key={item.id} href={item.href} aria-current={active ? 'page' : undefined} className={`group relative flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm font-medium transition-all ${active ? 'bg-cyan-400/10 text-cyan-200 ring-1 ring-cyan-300/15' : 'text-slate-400 hover:bg-white/[.045] hover:text-slate-100'}`}>
                {active && <span className="absolute left-0 h-5 w-0.5 rounded-r-full bg-cyan-300" />}
                <Icon className={`h-[18px] w-[18px] ${active ? 'text-cyan-300' : 'text-slate-500 group-hover:text-slate-300'}`} />
                <span>{item.label}</span>
                {!!badge && <span className="ml-auto min-w-5 rounded-full bg-rose-400/15 px-1.5 py-0.5 text-center text-[10px] font-bold text-rose-300 ring-1 ring-rose-400/20">{badge}</span>}
                {active && !badge && <ChevronRight className="ml-auto h-3.5 w-3.5 text-cyan-400/70" />}
              </Link>
            )
          })}
        </nav>

        <div className="rounded-2xl border border-white/8 bg-white/[.025] p-3">
          <div className="mb-3 flex items-center gap-3 px-1">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-lime-400/10 text-xs font-bold text-lime-300 ring-1 ring-lime-400/15">{userType === 'admin' ? 'AD' : 'OP'}</div>
            <div className="min-w-0"><p className="truncate text-sm font-medium text-slate-200">{userType === 'admin' ? 'Administrator' : 'Operator'}</p><p className="text-[11px] text-slate-500">Secure session</p></div>
          </div>
          <button onClick={handleLogout} className="flex w-full items-center gap-2 rounded-xl px-3 py-2 text-sm text-slate-400 transition-colors hover:bg-rose-400/10 hover:text-rose-300">
            <LogOut className="h-4 w-4" /> Sign out
          </button>
        </div>
      </aside>
    </>
  )
}

function Brand({ compact = false }: { compact?: boolean }) {
  return <div className="flex items-center gap-3"><div className={`${compact ? 'h-9 w-9' : 'h-10 w-10'} relative flex items-center justify-center rounded-xl bg-cyan-300 text-[#061016] shadow-[0_0_30px_rgba(34,211,238,.18)]`}><Activity className="h-5 w-5" /><span className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border-2 border-[#080d15] bg-lime-400" /></div><div><p className="text-[15px] font-bold tracking-tight text-white">WatchTower</p>{!compact && <p className="text-[10px] font-medium uppercase tracking-[.18em] text-slate-500">Operations</p>}</div></div>
}

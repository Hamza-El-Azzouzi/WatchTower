'use client'

import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { Activity, Home, Database, Zap, Settings, Bell, HelpCircle, Shield, Key, LogOut, Users, Lock, FileText } from 'lucide-react'
import { useEffect, useState } from 'react'
import { getAlerts } from '@/lib/alerts-api'

export function Sidebar() {
  const pathname = usePathname()
  const router = useRouter()
  const [alertCount, setAlertCount] = useState(0)
  const [userType, setUserType] = useState<string | null>(null)
  
  const isActive = (href: string) => pathname === href || pathname.startsWith(href + '/')
  
  useEffect(() => {
    // Get user type from localStorage
    const type = localStorage.getItem('user_type')
    setUserType(type)
    
    // Only fetch alerts for regular users
    if (type !== 'admin') {
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
    }
  }, [])
  
  // Monitoring items for regular users
  const monitoringItems = [
    { icon: Home, label: 'Overview', href: '/', id: 'overview' },
    { icon: Activity, label: 'Servers', href: '/', id: 'servers' },
    { icon: Database, label: 'Databases', href: '/databases', id: 'databases' },
    { icon: FileText, label: 'Logs', href: '/logs', id: 'logs' },
    { icon: Zap, label: 'Performance', href: '/performance', id: 'performance' },
    { icon: Bell, label: 'Alerts', href: '/alerts', id: 'alerts', badge: alertCount },
  ]

  // Admin items for admin users
  const adminItems = [
    { icon: Shield, label: 'Admin Dashboard', href: '/admin', id: 'admin-overview' },
    { icon: Key, label: 'API Keys', href: '/admin/api-keys', id: 'api-keys' },
    { icon: Users, label: 'Manage Admins', href: '/admin/users', id: 'manage-admins' },
    { icon: Lock, label: 'Change Password', href: '/admin/change-password', id: 'change-password' },
  ]

  // Determine which items to show based on user type
  const navItems = userType === 'admin' ? adminItems : monitoringItems
  
  const handleLogout = () => {
    localStorage.removeItem('api_key')
    localStorage.removeItem('admin_token')
    localStorage.removeItem('user_type')
    const loginPath = userType === 'admin' ? '/admin-login' : '/login'
    router.push(loginPath)
  }
  
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
            <p className="text-xs text-muted-foreground">
              {userType === 'admin' ? 'Administration' : 'Monitoring'}
            </p>
          </div>
        </div>
        
        {/* Navigation */}
        <nav className="flex flex-col gap-2">
          {navItems.map((item) => {
            const Icon = item.icon
            const active = isActive(item.href)
            const badge = 'badge' in item ? (item.badge as number) : undefined
            const hasBadge = badge !== undefined && badge > 0
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
                    {badge}
                  </span>
                )}
                {active && !hasBadge && <div className="ml-auto w-2 h-2 rounded-full bg-sidebar-accent animate-pulse-soft" />}
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
        <button 
          onClick={handleLogout}
          className="flex items-center gap-3 px-4 py-3 rounded-lg text-red-500 hover:bg-red-500/10 transition-smooth group w-full"
        >
          <LogOut className="w-5 h-5" />
          <span className="text-sm font-medium">Logout</span>
        </button>
      </div>
    </aside>
  )
}

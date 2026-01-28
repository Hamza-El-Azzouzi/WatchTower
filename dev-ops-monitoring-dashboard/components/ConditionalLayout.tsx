'use client'

import { usePathname } from 'next/navigation'
import { Sidebar } from '@/components/Sidebar'

export function ConditionalLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  
  // Don't show sidebar on login pages
  const isLoginPage = pathname === '/login' || pathname === '/admin-login'

  if (isLoginPage) {
    return <>{children}</>
  }

  return (
    <>
      <Sidebar />
      <div className="ml-64">
        {children}
      </div>
    </>
  )
}

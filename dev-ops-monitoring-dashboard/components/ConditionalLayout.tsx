'use client'

import { usePathname } from 'next/navigation'
import { Sidebar } from '@/components/Sidebar'

export function ConditionalLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname()
  
  // Don't show sidebar on login and public pages
  const isPublicPage = pathname === '/login' || pathname === '/admin-login' || pathname === '/request' || pathname === '/ws-test'

  if (isPublicPage) {
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

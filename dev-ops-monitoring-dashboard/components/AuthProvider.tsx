'use client'

import { useEffect } from 'react'
import { useRouter, usePathname } from 'next/navigation'

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const router = useRouter()
  const pathname = usePathname()

  useEffect(() => {
    const userType = localStorage.getItem('user_type')
    const apiKey = localStorage.getItem('api_key')
    const adminToken = localStorage.getItem('admin_token')
    
    // Public routes
    const publicRoutes = ['/login', '/admin-login']
    
    if (publicRoutes.includes(pathname)) {
      return
    }
    
    // Admin routes - require admin token
    if (pathname.startsWith('/admin')) {
      if (userType !== 'admin' || !adminToken) {
        router.push('/admin-login')
      }
      return
    }

    // Regular monitoring routes - require API key OR admin token
    if (!apiKey && userType !== 'admin') {
      router.push('/login')
    }
  }, [pathname, router])

  return <>{children}</>
}

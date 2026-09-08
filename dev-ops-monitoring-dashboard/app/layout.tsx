import React from "react"
import type { Metadata } from 'next'
import { Analytics } from '@vercel/analytics/next'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { AuthProvider } from '@/components/AuthProvider'
import { ConditionalLayout } from '@/components/ConditionalLayout'
import { MetricsProvider } from '@/contexts/MetricsContext'
import './globals.css'

export const metadata: Metadata = {
  title: 'DevOps Monitoring System | Real-time Infrastructure Dashboard',
  description: 'Monitor your infrastructure with real-time metrics, historical charts, and instant alerts. Track CPU, memory, disk, and network usage across all your servers.',
  icons: {
    icon: [
      {
        url: '/favicon-32x32.png',
        media: '(prefers-color-scheme: light)',
      },
      {
        url: '/favicon-32x32.png',
        media: '(prefers-color-scheme: dark)',
      },
      {
        url: '/favicon.ico',
        type: 'image/svg+xml',
      },
    ],
    apple: '/apple-touch-icon.png',
  },
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" className="dark">
      <body className={`font-sans antialiased bg-background text-foreground`}>
        <ErrorBoundary>
          <AuthProvider>
            <MetricsProvider>
              <ConditionalLayout>
                {children}
              </ConditionalLayout>
            </MetricsProvider>
          </AuthProvider>
        </ErrorBoundary>
        <Analytics />
      </body>
    </html>
  )
}

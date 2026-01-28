import React from "react"
import type { Metadata } from 'next'
import { Geist, Geist_Mono } from 'next/font/google'
import { Analytics } from '@vercel/analytics/next'
import { ErrorBoundary } from '@/components/ErrorBoundary'
import { Sidebar } from '@/components/Sidebar'
import './globals.css'

const _geist = Geist({ subsets: ["latin"] });
const _geistMono = Geist_Mono({ subsets: ["latin"] });

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
          <Sidebar />
          <div className="ml-64">
            {children}
          </div>
        </ErrorBoundary>
        <Analytics />
      </body>
    </html>
  )
}

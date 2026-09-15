'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { Shield, Lock, User, AlertCircle } from 'lucide-react'

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export default function AdminLoginPage() {
  const router = useRouter()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError('')

    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/admin/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ username, password }),
      })

      if (response.ok) {
        const data = await response.json()
        // Store admin token and username
        localStorage.setItem('admin_token', data.token)
        localStorage.setItem('admin_username', data.username)
        localStorage.setItem('admin_full_name', data.full_name || '')
        localStorage.setItem('user_type', 'admin')
        
        // Redirect to admin dashboard
        router.push('/admin')
      } else {
        const errorData = await response.json()
        setError(errorData.error || 'Invalid username or password')
      }
    } catch (err) {
      setError('Connection error. Please make sure the server is running.')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-[#070b12] p-4 sm:p-8">
      <div className="pointer-events-none absolute inset-0 opacity-60" style={{ backgroundImage: 'linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.025) 1px, transparent 1px)', backgroundSize: '42px 42px' }} />
      <div className="pointer-events-none absolute -right-32 top-[-12rem] h-[34rem] w-[34rem] rounded-full bg-lime-400/8 blur-3xl" />
      <div className="relative z-10 w-full max-w-md">
        {/* Logo and Title */}
        <div className="text-center mb-8">
          <div className="mb-5 inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-lime-300 text-[#0b1306] shadow-[0_0_45px_rgba(132,204,22,.15)]">
            <Shield className="h-6 w-6" />
          </div>
          <p className="eyebrow mb-2">Privileged access</p>
          <h1 className="text-3xl font-semibold tracking-[-.04em] text-white">Administration</h1>
          <p className="mt-2 text-sm text-slate-400">Manage agents, access, and platform policy.</p>
        </div>

        {/* Login Form */}
        <div className="surface-panel rounded-[24px] p-6 sm:p-8">
          {error && (
            <div className="mb-6 p-4 bg-red-500/10 border border-red-500/20 rounded-lg flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
              <p className="text-sm text-red-500">{error}</p>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-6">
            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                <User className="w-4 h-4 inline mr-2" />
                Username
              </label>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full rounded-xl border border-white/10 bg-black/20 px-4 py-3 text-foreground placeholder:text-slate-600 focus:border-lime-300/40 focus:ring-2 focus:ring-lime-300/15"
                placeholder="Enter your username"
                required
                autoComplete="username"
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-foreground mb-2">
                <Lock className="w-4 h-4 inline mr-2" />
                Password
              </label>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full rounded-xl border border-white/10 bg-black/20 px-4 py-3 text-foreground placeholder:text-slate-600 focus:border-lime-300/40 focus:ring-2 focus:ring-lime-300/15"
                placeholder="Enter your password"
                required
                autoComplete="current-password"
              />
            </div>

            <button
              type="submit"
              disabled={loading}
              className="w-full rounded-xl bg-lime-300 px-4 py-3 font-semibold text-[#0b1306] transition hover:bg-lime-200 disabled:cursor-not-allowed disabled:opacity-50"
            >
              {loading ? (
                <span className="flex items-center justify-center gap-2">
                  <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Signing in...
                </span>
              ) : (
                'Sign In'
              )}
            </button>
          </form>

          <div className="mt-6 pt-6 border-t border-card-border">
            <button
              onClick={() => router.push('/login')}
              className="w-full text-center text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Regular user? Sign in with API key →
            </button>
          </div>
        </div>

        <p className="mt-6 text-center text-xs text-muted-foreground">
          The first administrator is configured by the server operator.
        </p>
      </div>
    </div>
  )
}

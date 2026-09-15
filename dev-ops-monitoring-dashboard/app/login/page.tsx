'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { Key, Lock, AlertCircle, Loader } from 'lucide-react'

const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

export default function LoginPage() {
  const router = useRouter()
  const [apiKey, setApiKey] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)

    try {
      // Validate the API key using the validation endpoint
      const response = await fetch(`${API_BASE_URL}/api/v1/auth/validate`, {
        headers: {
          'X-API-Key': apiKey
        }
      })

      if (response.ok) {
        const data = await response.json()
        if (data.valid) {
          // Store the API key
          localStorage.setItem('api_key', apiKey)
          localStorage.setItem('user_type', 'user')
          
          // Redirect to dashboard
          router.push('/')
        } else {
          setError('Invalid API key. Please check and try again.')
        }
      } else if (response.status === 400 || response.status === 401 || response.status === 403) {
        const errorData = await response.json().catch(() => ({}))
        setError(errorData.error || 'Invalid API key. Please check and try again.')
      } else {
        setError('Unable to verify API key. Please ensure the server is running.')
      }
    } catch (err) {
      setError(`Connection error. Please ensure the API server is running at ${API_BASE_URL}`)
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="relative flex min-h-screen items-center justify-center overflow-hidden bg-[#070b12] p-4 sm:p-8">
      <div className="pointer-events-none absolute inset-0 opacity-60" style={{ backgroundImage: 'linear-gradient(rgba(255,255,255,.025) 1px, transparent 1px), linear-gradient(90deg, rgba(255,255,255,.025) 1px, transparent 1px)', backgroundSize: '42px 42px' }} />
      <div className="pointer-events-none absolute -left-32 top-[-12rem] h-[34rem] w-[34rem] rounded-full bg-cyan-400/10 blur-3xl" />
      <div className="relative z-10 w-full max-w-md">
        {/* Logo/Header */}
        <div className="text-center mb-8">
          <div className="mb-5 inline-flex h-14 w-14 items-center justify-center rounded-2xl bg-cyan-300 text-[#061016] shadow-[0_0_45px_rgba(34,211,238,.2)]">
            <Lock className="h-6 w-6" />
          </div>
          <p className="eyebrow mb-2">WatchTower operations</p>
          <h1 className="text-3xl font-semibold tracking-[-.04em] text-white">Welcome back</h1>
          <p className="mt-2 text-sm text-slate-400">Use your workspace key to open the command center.</p>
        </div>

        {/* Login Card */}
        <div className="surface-panel rounded-[24px] p-6 sm:p-8">
          <form onSubmit={handleLogin} className="space-y-6">
            {/* Error Message */}
            {error && (
              <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-start gap-3">
                <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-semibold text-red-300 mb-1">We couldn&apos;t sign you in</h3>
                  <p className="text-sm text-red-200">{error}</p>
                </div>
              </div>
            )}

            {/* API Key Input */}
            <div className="space-y-2">
              <label htmlFor="apiKey" className="block text-sm font-medium text-gray-300">
                API Key
              </label>
              <div className="relative">
                <div className="absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none">
                  <Key className="h-5 w-5 text-gray-500" />
                </div>
                <input
                  id="apiKey"
                type="password"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="msk_xxxxxxxxxxxxxxxxxxxxxxxxx"
                  className="block w-full rounded-xl border border-white/10 bg-black/20 py-3 pl-12 pr-4 text-white placeholder-slate-600 transition focus:border-cyan-300/40 focus:ring-2 focus:ring-cyan-300/15"
                  required
                  autoFocus
                  disabled={loading}
                />
              </div>
              <p className="text-xs text-gray-500">
                Your API key is used to authenticate and filter your monitored servers
              </p>
            </div>

            {/* Submit Button */}
            <button
              type="submit"
              disabled={loading || !apiKey}
              className="flex w-full items-center justify-center gap-2 rounded-xl bg-cyan-300 px-4 py-3 font-semibold text-[#061016] transition hover:bg-cyan-200 disabled:cursor-not-allowed disabled:bg-slate-700 disabled:text-slate-400"
            >
              {loading ? (
                <>
                  <Loader className="w-5 h-5 animate-spin" />
                  Authenticating...
                </>
              ) : (
                <>
                  <Lock className="w-5 h-5" />
                  Access Dashboard
                </>
              )}
            </button>
          </form>

          {/* Help Text */}
          <div className="mt-6 pt-6 border-t border-gray-700 space-y-3">
            <p className="text-sm text-gray-400 text-center">
              Don&apos;t have an API key?{' '}
              <a href="/request" className="text-blue-400 hover:text-blue-300 font-medium">
                Request agent quota
              </a>
            </p>
          </div>
        </div>

        {/* Footer Info */}
        <div className="mt-8 text-center">
          <p className="text-sm text-gray-500">
            Tenant-scoped monitoring access
          </p>
          <div className="mt-6 pt-6 border-t border-card-border">
            <button
              onClick={() => router.push('/admin-login')}
              className="w-full text-center text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Admin user? Sign in here →
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}

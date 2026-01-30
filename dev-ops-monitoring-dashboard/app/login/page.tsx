'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { Key, Lock, AlertCircle, Loader } from 'lucide-react'

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
      const response = await fetch('http://localhost:8080/api/v1/auth/validate', {
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
      setError('Connection error. Please ensure the API server is running at http://localhost:8080')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 flex items-center justify-center p-4">
      <div className="max-w-md w-full">
        {/* Logo/Header */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-500/20 rounded-2xl mb-4 border border-blue-500/30">
            <Lock className="w-8 h-8 text-blue-400" />
          </div>
          <h1 className="text-3xl font-bold text-white mb-2">DevOps Monitor</h1>
          <p className="text-gray-400">Enter your API key to access the dashboard</p>
        </div>

        {/* Login Card */}
        <div className="bg-gray-800/50 backdrop-blur-xl rounded-2xl border border-gray-700 p-8 shadow-2xl">
          <form onSubmit={handleLogin} className="space-y-6">
            {/* Error Message */}
            {error && (
              <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-start gap-3">
                <AlertCircle className="w-5 h-5 text-red-400 flex-shrink-0 mt-0.5" />
                <div>
                  <h3 className="font-semibold text-red-300 mb-1">Authentication Failed</h3>
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
                  type="text"
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="msk_xxxxxxxxxxxxxxxxxxxxxxxxx"
                  className="block w-full pl-12 pr-4 py-3 bg-gray-900/50 border border-gray-600 rounded-xl text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
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
              className="w-full bg-blue-500 hover:bg-blue-600 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-semibold py-3 px-4 rounded-xl transition-all flex items-center justify-center gap-2"
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
            🔒 Your API key is stored locally and never shared
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

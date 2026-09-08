'use client'

import { useState, useEffect } from 'react'
import { Plus, Key, Trash2, Copy, Check, AlertCircle, RefreshCw, Clock, Shield, Activity } from 'lucide-react'
import { listApiKeys, createApiKey, revokeApiKey, type ApiKey, type CreateApiKeyRequest } from '@/lib/api-keys-api'

export default function ApiKeysPage() {
  const [keys, setKeys] = useState<ApiKey[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [showCreateModal, setShowCreateModal] = useState(false)
  const [newKeyResponse, setNewKeyResponse] = useState<{ key: string; name: string } | null>(null)
  const [copiedKey, setCopiedKey] = useState(false)

  useEffect(() => {
    loadKeys()
  }, [])

  const loadKeys = async () => {
    try {
      setLoading(true)
      setError(null)
      const data = await listApiKeys()
      setKeys(data)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load API keys')
    } finally {
      setLoading(false)
    }
  }

  const handleCopyKey = async (key: string) => {
    await navigator.clipboard.writeText(key)
    setCopiedKey(true)
    setTimeout(() => setCopiedKey(false), 2000)
  }

  const handleRevokeKey = async (keyId: number, keyName: string) => {
    if (!confirm(`Are you sure you want to revoke the API key "${keyName}"? Agents using this key will stop working.`)) {
      return
    }

    try {
      await revokeApiKey(keyId)
      await loadKeys()
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to revoke API key')
    }
  }

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never'
    return new Date(dateString).toLocaleString()
  }

  const isExpired = (expiresAt: string | null) => {
    if (!expiresAt) return false
    return new Date(expiresAt) < new Date()
  }

  const getKeyStatus = (key: ApiKey) => {
    if (key.revoked) return { label: 'Revoked', color: 'bg-red-500' }
    if (isExpired(key.expires_at)) return { label: 'Expired', color: 'bg-orange-500' }
    return { label: 'Active', color: 'bg-green-500' }
  }

  return (
    <div className="min-h-screen bg-background p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-foreground flex items-center gap-3">
              <Shield className="w-8 h-8 text-primary" />
              API Key Management
            </h1>
            <p className="text-muted-foreground mt-2">
              Manage authentication tokens for monitoring agents
            </p>
          </div>
          <div className="flex gap-3">
            <button
              onClick={loadKeys}
              className="px-4 py-2 bg-sidebar-accent/20 text-foreground rounded-lg hover:bg-sidebar-accent/30 transition-colors flex items-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Refresh
            </button>
            <button
              onClick={() => setShowCreateModal(true)}
              className="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors flex items-center gap-2"
            >
              <Plus className="w-4 h-4" />
              Generate New Key
            </button>
          </div>
        </div>

        {/* Error Alert */}
        {error && (
          <div className="mb-6 p-4 bg-red-500/10 border border-red-500/20 rounded-lg flex items-center gap-3">
            <AlertCircle className="w-5 h-5 text-red-500" />
            <p className="text-red-500">{error}</p>
          </div>
        )}

        {/* Keys List */}
        {loading ? (
          <div className="flex items-center justify-center py-20">
            <RefreshCw className="w-8 h-8 animate-spin text-primary" />
          </div>
        ) : keys.length === 0 ? (
          <div className="text-center py-20">
            <Key className="w-16 h-16 text-muted-foreground mx-auto mb-4" />
            <h3 className="text-xl font-semibold text-foreground mb-2">No API Keys</h3>
            <p className="text-muted-foreground mb-6">Get started by creating your first API key</p>
            <button
              onClick={() => setShowCreateModal(true)}
              className="px-6 py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              Generate API Key
            </button>
          </div>
        ) : (
          <div className="space-y-4">
            {keys.map((key) => {
              const status = getKeyStatus(key)
              return (
                <div
                  key={key.id}
                  className="bg-card border border-card-border rounded-lg p-6 hover:border-primary/50 transition-colors"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center gap-3 mb-2">
                        <Key className="w-5 h-5 text-primary" />
                        <h3 className="text-lg font-semibold text-foreground">{key.name}</h3>
                        <span className={`px-2 py-1 rounded-full text-xs font-medium text-white ${status.color}`}>
                          {status.label}
                        </span>
                      </div>
                      {key.description && (
                        <p className="text-sm text-muted-foreground mb-3">{key.description}</p>
                      )}
                      <div className="flex items-center gap-2 mb-3">
                        <code className="px-3 py-2 bg-sidebar-accent/20 rounded font-mono text-sm text-foreground">
                          {key.key}
                        </code>
                      </div>
                      <div className="grid grid-cols-2 gap-4 text-sm">
                        <div>
                          <span className="text-muted-foreground">Created:</span>
                          <span className="ml-2 text-foreground">{formatDate(key.created_at)}</span>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Created By:</span>
                          <span className="ml-2 text-foreground">{key.created_by}</span>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Last Used:</span>
                          <span className="ml-2 text-foreground">{formatDate(key.last_used_at)}</span>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Expires:</span>
                          <span className={`ml-2 ${isExpired(key.expires_at) ? 'text-orange-500' : 'text-foreground'}`}>
                            {key.expires_at ? formatDate(key.expires_at) : 'Never'}
                          </span>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Agent Limit:</span>
                          <span className="ml-2 text-foreground">
                            {key.used_by_agents?.length || 0}
                            {key.max_agents ? ` / ${key.max_agents}` : ' / Unlimited'}
                          </span>
                        </div>
                      </div>
                      
                      {/* Agents Using This Key */}
                      {key.used_by_agents && key.used_by_agents.length > 0 && (
                        <div className="mt-4 pt-4 border-t border-card-border">
                          <div className="flex items-center gap-2 mb-2">
                            <Activity className="w-4 h-4 text-primary" />
                            <span className="text-sm font-semibold text-foreground">Agents Using This Key</span>
                          </div>
                          <div className="flex flex-wrap gap-2">
                            {key.used_by_agents.map((agentId, idx) => (
                              <span
                                key={idx}
                                className="px-2 py-1 bg-sidebar-accent/20 text-foreground rounded text-xs font-mono"
                              >
                                {agentId}
                              </span>
                            ))}
                          </div>
                        </div>
                      )}
                    </div>
                    <div className="flex gap-2 ml-4">
                      {!key.revoked && (
                        <button
                          onClick={() => handleRevokeKey(key.id, key.name)}
                          className="p-2 bg-red-500/10 text-red-500 rounded-lg hover:bg-red-500/20 transition-colors"
                          title="Revoke Key"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              )
            })}
          </div>
        )}

        {/* Create Key Modal */}
        {showCreateModal && (
          <CreateKeyModal
            onClose={() => {
              setShowCreateModal(false)
              setNewKeyResponse(null)
            }}
            onSuccess={(response) => {
              setNewKeyResponse(response)
              loadKeys()
            }}
            newKeyResponse={newKeyResponse}
            onCopyKey={handleCopyKey}
            copiedKey={copiedKey}
          />
        )}
      </div>
    </div>
  )
}

function CreateKeyModal({
  onClose,
  onSuccess,
  newKeyResponse,
  onCopyKey,
  copiedKey,
}: {
  onClose: () => void
  onSuccess: (response: { key: string; name: string }) => void
  newKeyResponse: { key: string; name: string } | null
  onCopyKey: (key: string) => void
  copiedKey: boolean
}) {
  const [formData, setFormData] = useState<CreateApiKeyRequest>({
    name: '',
    description: '',
    expires_in_days: 90,
    max_agents: undefined,
  })
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setLoading(true)
    setError(null)

    try {
      const response = await createApiKey(formData)
      onSuccess(response)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create API key')
    } finally {
      setLoading(false)
    }
  }

  if (newKeyResponse) {
    return (
      <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
        <div className="bg-card border border-card-border rounded-lg p-6 max-w-2xl w-full">
          <h2 className="text-2xl font-bold text-foreground mb-4 flex items-center gap-2">
            <Check className="w-6 h-6 text-green-500" />
            API Key Created Successfully
          </h2>
          <div className="mb-6 p-4 bg-yellow-500/10 border border-yellow-500/20 rounded-lg">
            <p className="text-yellow-500 font-semibold mb-2">⚠️ Important: Save this key now!</p>
            <p className="text-sm text-muted-foreground">
              This is the only time you&apos;ll be able to see the full key. Make sure to copy it and store it securely.
            </p>
          </div>
          <div className="mb-6">
            <label className="block text-sm font-medium text-muted-foreground mb-2">Key Name</label>
            <p className="text-foreground font-semibold">{newKeyResponse.name}</p>
          </div>
          <div className="mb-6">
            <label className="block text-sm font-medium text-muted-foreground mb-2">API Key</label>
            <div className="flex gap-2">
              <code className="flex-1 px-4 py-3 bg-sidebar-accent/20 rounded font-mono text-sm text-foreground break-all">
                {newKeyResponse.key}
              </code>
              <button
                onClick={() => onCopyKey(newKeyResponse.key)}
                className="px-4 py-2 bg-primary text-primary-foreground rounded hover:bg-primary/90 transition-colors flex items-center gap-2"
              >
                {copiedKey ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
                {copiedKey ? 'Copied!' : 'Copy'}
              </button>
            </div>
          </div>
          <div className="flex justify-end">
            <button
              onClick={onClose}
              className="px-6 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors"
            >
              Done
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-card border border-card-border rounded-lg p-6 max-w-md w-full">
        <h2 className="text-2xl font-bold text-foreground mb-4">Generate New API Key</h2>
        
        {error && (
          <div className="mb-4 p-3 bg-red-500/10 border border-red-500/20 rounded flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-red-500" />
            <p className="text-sm text-red-500">{error}</p>
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-foreground mb-2">
              Key Name *
            </label>
            <input
              type="text"
              required
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              className="w-full px-3 py-2 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-primary text-foreground"
              placeholder="e.g., production-agent-1"
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-2">
              Description
            </label>
            <textarea
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              className="w-full px-3 py-2 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-primary text-foreground"
              placeholder="What is this key for?"
              rows={3}
            />
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-2">
              Expires In (Days)
            </label>
            <select
              value={formData.expires_in_days || ''}
              onChange={(e) => setFormData({ ...formData, expires_in_days: e.target.value ? parseInt(e.target.value) : undefined })}
              className="w-full px-3 py-2 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-primary text-foreground"
            >
              <option value="">Never</option>
              <option value="7">7 days</option>
              <option value="30">30 days</option>
              <option value="90">90 days (recommended)</option>
              <option value="180">180 days</option>
              <option value="365">365 days</option>
            </select>
          </div>

          <div>
            <label className="block text-sm font-medium text-foreground mb-2">
              Maximum Agents (Optional)
            </label>
            <input
              type="number"
              min="1"
              value={formData.max_agents || ''}
              onChange={(e) => setFormData({ ...formData, max_agents: e.target.value ? parseInt(e.target.value) : undefined })}
              className="w-full px-3 py-2 bg-background border border-input rounded-lg focus:outline-none focus:ring-2 focus:ring-primary text-foreground"
              placeholder="Unlimited if not specified"
            />
            <p className="text-xs text-muted-foreground mt-1">
              Limit how many agents can use this key. Leave empty for unlimited.
            </p>
          </div>

          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-2 bg-sidebar-accent/20 text-foreground rounded-lg hover:bg-sidebar-accent/30 transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={loading}
              className="flex-1 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors disabled:opacity-50"
            >
              {loading ? 'Creating...' : 'Generate Key'}
            </button>
          </div>
        </form>
      </div>
    </div>
  )
}

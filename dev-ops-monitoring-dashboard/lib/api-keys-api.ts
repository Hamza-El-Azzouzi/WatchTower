const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface ApiKey {
  id: number
  key: string
  name: string
  description: string | null
  created_at: string
  expires_at: string | null
  last_used_at: string | null
  revoked: boolean
  revoked_at: string | null
  created_by: string
  max_agents: number | null
  used_by_agents: string[]
}

export interface CreateApiKeyRequest {
  name: string
  description?: string
  expires_in_days?: number
  max_agents?: number
}

export interface CreateApiKeyResponse {
  key: string
  name: string
  expires_at: string | null
}

export async function createApiKey(request: CreateApiKeyRequest): Promise<CreateApiKeyResponse> {
  const response = await fetch(`${API_BASE_URL}/api/v1/auth/keys`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(request),
  })

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to create API key' }))
    throw new Error(errorData.error || `HTTP ${response.status}`)
  }

  return response.json()
}

export async function listApiKeys(): Promise<ApiKey[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/auth/keys`)

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to fetch API keys' }))
    throw new Error(errorData.error || `HTTP ${response.status}`)
  }

  return response.json()
}

export async function revokeApiKey(keyId: number): Promise<void> {
  const response = await fetch(`${API_BASE_URL}/api/v1/auth/keys/${keyId}`, {
    method: 'DELETE',
  })

  if (!response.ok) {
    const errorData = await response.json().catch(() => ({ error: 'Failed to revoke API key' }))
    throw new Error(errorData.error || `HTTP ${response.status}`)
  }
}

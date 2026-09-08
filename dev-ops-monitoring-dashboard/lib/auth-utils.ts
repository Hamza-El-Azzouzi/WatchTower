// Helper function to get auth headers
export function getAuthHeaders(): HeadersInit {
  // Check if user is admin
  const userType = typeof window !== 'undefined' ? localStorage.getItem('user_type') : null
  
  if (userType === 'admin') {
    const adminToken = typeof window !== 'undefined' ? localStorage.getItem('admin_token') : null
    return adminToken ? { 'X-Admin-Token': adminToken } : {}
  }
  
  // Regular user with API key
  const apiKey = typeof window !== 'undefined' ? localStorage.getItem('api_key') : null
  return apiKey ? { 'X-API-Key': apiKey } : {}
}

export function getRealtimeToken(): string | null {
  if (typeof window === 'undefined') return null
  return localStorage.getItem('user_type') === 'admin'
    ? localStorage.getItem('admin_token')
    : localStorage.getItem('api_key')
}

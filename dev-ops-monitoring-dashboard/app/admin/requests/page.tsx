'use client'

import { useState, useEffect, useCallback } from 'react'
import { useRouter } from 'next/navigation'
import { 
  Users, 
  Building2, 
  Mail, 
  Phone, 
  Server, 
  Clock, 
  CheckCircle, 
  XCircle, 
  MessageSquare,
  ChevronDown,
  Loader2,
  ArrowLeft,
  Filter,
  RefreshCw
} from 'lucide-react'

interface AgentRequest {
  id: number
  company_name: string
  contact_name: string
  email: string
  phone: string | null
  agents_requested: number
  use_case: string
  message: string | null
  status: 'pending' | 'approved' | 'rejected' | 'contacted'
  created_at: string
  updated_at: string
  reviewed_by: string | null
  reviewed_at: string | null
  notes: string | null
}

const STATUS_COLORS = {
  pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  approved: 'bg-green-500/20 text-green-400 border-green-500/30',
  rejected: 'bg-red-500/20 text-red-400 border-red-500/30',
  contacted: 'bg-blue-500/20 text-blue-400 border-blue-500/30',
}

const STATUS_ICONS = {
  pending: Clock,
  approved: CheckCircle,
  rejected: XCircle,
  contacted: MessageSquare,
}

export default function AgentRequestsPage() {
  const router = useRouter()
  const [requests, setRequests] = useState<AgentRequest[]>([])
  const [loading, setLoading] = useState(true)
  const [statusFilter, setStatusFilter] = useState<string>('')
  const [selectedRequest, setSelectedRequest] = useState<AgentRequest | null>(null)
  const [updating, setUpdating] = useState(false)
  const [updateNotes, setUpdateNotes] = useState('')
  const [error, setError] = useState<string | null>(null)

  const getToken = () => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('admin_token')
    }
    return null
  }

  const fetchRequests = useCallback(async () => {
    setLoading(true)
    setError(null)
    
    try {
      const token = getToken()
      if (!token) {
        router.push('/admin-login')
        return
      }

      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'
      const url = statusFilter 
        ? `${apiUrl}/api/v1/requests/agents?status=${statusFilter}`
        : `${apiUrl}/api/v1/requests/agents`
      
      const response = await fetch(url, {
        headers: {
          'Authorization': `Bearer ${token}`,
        },
      })

      if (response.status === 401) {
        router.push('/admin-login')
        return
      }

      if (!response.ok) {
        throw new Error('Failed to fetch requests')
      }

      const data = await response.json()
      setRequests(data.requests || [])
    } catch (err) {
      setError('Failed to load requests. Please try again.')
      console.error(err)
    } finally {
      setLoading(false)
    }
  }, [statusFilter, router])

  useEffect(() => {
    fetchRequests()
  }, [fetchRequests])

  const updateRequestStatus = async (requestId: number, newStatus: string) => {
    setUpdating(true)
    
    try {
      const token = getToken()
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'
      
      const response = await fetch(`${apiUrl}/api/v1/requests/agents/${requestId}/status`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${token}`,
        },
        body: JSON.stringify({
          status: newStatus,
          notes: updateNotes || undefined,
          reviewed_by: 'admin', // Could get from token
        }),
      })

      if (response.ok) {
        setSelectedRequest(null)
        setUpdateNotes('')
        fetchRequests()
      } else {
        const data = await response.json()
        setError(data.error || 'Failed to update request')
      }
    } catch (err) {
      setError('Failed to update request')
    } finally {
      setUpdating(false)
    }
  }

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div className="flex items-center gap-4">
            <button
              onClick={() => router.push('/admin')}
              className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
            >
              <ArrowLeft className="w-5 h-5 text-gray-400" />
            </button>
            <div>
              <h1 className="text-2xl font-bold text-white flex items-center gap-3">
                <Users className="w-7 h-7 text-blue-400" />
                Agent Requests
              </h1>
              <p className="text-gray-400 text-sm mt-1">
                Manage incoming agent quota requests from users
              </p>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <button
              onClick={fetchRequests}
              className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
              title="Refresh"
            >
              <RefreshCw className={`w-5 h-5 text-gray-400 ${loading ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className="bg-gray-800/50 rounded-xl p-4 mb-6 flex items-center gap-4 border border-gray-700/50">
          <Filter className="w-5 h-5 text-gray-400" />
          <span className="text-gray-400 text-sm">Filter by status:</span>
          <div className="flex gap-2">
            {['', 'pending', 'contacted', 'approved', 'rejected'].map((status) => (
              <button
                key={status}
                onClick={() => setStatusFilter(status)}
                className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-all ${
                  statusFilter === status
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-700/50 text-gray-300 hover:bg-gray-600/50'
                }`}
              >
                {status || 'All'}
              </button>
            ))}
          </div>
          <span className="ml-auto text-sm text-gray-500">
            {requests.length} request{requests.length !== 1 ? 's' : ''}
          </span>
        </div>

        {/* Error */}
        {error && (
          <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4 mb-6 text-red-400">
            {error}
          </div>
        )}

        {/* Loading */}
        {loading && (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="w-8 h-8 animate-spin text-blue-400" />
          </div>
        )}

        {/* Empty State */}
        {!loading && requests.length === 0 && (
          <div className="text-center py-12 bg-gray-800/30 rounded-xl border border-gray-700/50">
            <Users className="w-12 h-12 text-gray-600 mx-auto mb-4" />
            <p className="text-gray-400">No requests found</p>
            <p className="text-gray-500 text-sm mt-1">
              {statusFilter ? `Try a different filter` : 'Requests will appear here when users submit them'}
            </p>
          </div>
        )}

        {/* Requests Grid */}
        {!loading && requests.length > 0 && (
          <div className="grid gap-4">
            {requests.map((request) => {
              const StatusIcon = STATUS_ICONS[request.status]
              return (
                <div
                  key={request.id}
                  className="bg-gray-800/50 rounded-xl border border-gray-700/50 hover:border-gray-600/50 transition-all"
                >
                  <div className="p-6">
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3 mb-3">
                          <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                            <Building2 className="w-5 h-5 text-blue-400" />
                            {request.company_name}
                          </h3>
                          <span className={`px-2.5 py-1 rounded-full text-xs font-medium border ${STATUS_COLORS[request.status]}`}>
                            <StatusIcon className="w-3 h-3 inline mr-1" />
                            {request.status.charAt(0).toUpperCase() + request.status.slice(1)}
                          </span>
                        </div>
                        
                        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
                          <div>
                            <p className="text-gray-500">Contact</p>
                            <p className="text-white">{request.contact_name}</p>
                          </div>
                          <div>
                            <p className="text-gray-500 flex items-center gap-1">
                              <Mail className="w-3 h-3" /> Email
                            </p>
                            <a href={`mailto:${request.email}`} className="text-blue-400 hover:text-blue-300">
                              {request.email}
                            </a>
                          </div>
                          <div>
                            <p className="text-gray-500 flex items-center gap-1">
                              <Server className="w-3 h-3" /> Agents
                            </p>
                            <p className="text-white font-semibold">{request.agents_requested}</p>
                          </div>
                          <div>
                            <p className="text-gray-500">Use Case</p>
                            <p className="text-white">{request.use_case}</p>
                          </div>
                        </div>

                        {request.phone && (
                          <div className="mt-3 flex items-center gap-2 text-sm text-gray-400">
                            <Phone className="w-4 h-4" />
                            {request.phone}
                          </div>
                        )}

                        {request.message && (
                          <div className="mt-3 p-3 bg-gray-700/30 rounded-lg">
                            <p className="text-sm text-gray-300">{request.message}</p>
                          </div>
                        )}

                        <div className="mt-4 flex items-center gap-4 text-xs text-gray-500">
                          <span>Submitted: {formatDate(request.created_at)}</span>
                          {request.reviewed_by && (
                            <span>Reviewed by: {request.reviewed_by}</span>
                          )}
                        </div>

                        {request.notes && (
                          <div className="mt-2 text-xs text-gray-400">
                            <span className="text-gray-500">Notes:</span> {request.notes}
                          </div>
                        )}
                      </div>

                      {/* Actions */}
                      <div className="ml-4">
                        <div className="relative">
                          <button
                            onClick={() => setSelectedRequest(selectedRequest?.id === request.id ? null : request)}
                            className="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg text-sm font-medium text-white flex items-center gap-2 transition-colors"
                          >
                            Update Status
                            <ChevronDown className={`w-4 h-4 transition-transform ${selectedRequest?.id === request.id ? 'rotate-180' : ''}`} />
                          </button>
                        </div>
                      </div>
                    </div>

                    {/* Status Update Panel */}
                    {selectedRequest?.id === request.id && (
                      <div className="mt-4 pt-4 border-t border-gray-700">
                        <div className="flex items-start gap-4">
                          <div className="flex-1">
                            <label className="block text-sm text-gray-400 mb-2">Add notes (optional)</label>
                            <textarea
                              value={updateNotes}
                              onChange={(e) => setUpdateNotes(e.target.value)}
                              placeholder="Add any notes about this request..."
                              className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm resize-none"
                              rows={2}
                            />
                          </div>
                          <div className="flex flex-col gap-2">
                            <button
                              onClick={() => updateRequestStatus(request.id, 'contacted')}
                              disabled={updating}
                              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 rounded-lg text-sm font-medium text-white transition-colors"
                            >
                              Mark Contacted
                            </button>
                            <button
                              onClick={() => updateRequestStatus(request.id, 'approved')}
                              disabled={updating}
                              className="px-4 py-2 bg-green-600 hover:bg-green-700 disabled:bg-gray-600 rounded-lg text-sm font-medium text-white transition-colors"
                            >
                              Approve
                            </button>
                            <button
                              onClick={() => updateRequestStatus(request.id, 'rejected')}
                              disabled={updating}
                              className="px-4 py-2 bg-red-600 hover:bg-red-700 disabled:bg-gray-600 rounded-lg text-sm font-medium text-white transition-colors"
                            >
                              Reject
                            </button>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>
    </div>
  )
}

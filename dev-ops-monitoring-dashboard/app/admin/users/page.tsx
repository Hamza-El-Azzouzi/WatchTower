'use client'

import { useEffect, useState } from 'react'
import { Users, Plus, UserCheck, UserX, AlertCircle, Loader2 } from 'lucide-react'

interface AdminUser {
  id: number
  username: string
  email: string | null
  full_name: string | null
  created_at: string
  last_login_at: string | null
  is_active: boolean
}

export default function ManageAdminsPage() {
  const [admins, setAdmins] = useState<AdminUser[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const [showCreateForm, setShowCreateForm] = useState(false)
  const [createLoading, setCreateLoading] = useState(false)
  const [formData, setFormData] = useState({
    username: '',
    password: '',
    email: '',
    full_name: '',
  })

  useEffect(() => {
    fetchAdmins()
  }, [])

  const fetchAdmins = async () => {
    try {
      const adminToken = localStorage.getItem('admin_token')
      const response = await fetch('http://localhost:8080/api/v1/admin/users', {
        headers: {
          'X-Admin-Token': adminToken || '',
        },
      })

      if (response.ok) {
        const data = await response.json()
        setAdmins(data)
      } else {
        setError('Failed to load admin users')
      }
    } catch (err) {
      setError('Network error. Please try again.')
    } finally {
      setLoading(false)
    }
  }

  const handleCreateAdmin = async (e: React.FormEvent) => {
    e.preventDefault()
    setError('')
    setCreateLoading(true)

    try {
      const adminToken = localStorage.getItem('admin_token')
      const response = await fetch('http://localhost:8080/api/v1/admin/users', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Admin-Token': adminToken || '',
        },
        body: JSON.stringify(formData),
      })

      if (response.ok) {
        setShowCreateForm(false)
        setFormData({ username: '', password: '', email: '', full_name: '' })
        fetchAdmins()
      } else {
        const data = await response.json()
        setError(data.error || 'Failed to create admin')
      }
    } catch (err) {
      setError('Network error. Please try again.')
    } finally {
      setCreateLoading(false)
    }
  }

  const handleToggleStatus = async (username: string, isActive: boolean) => {
    const endpoint = isActive ? 'deactivate' : 'activate'
    
    try {
      const adminToken = localStorage.getItem('admin_token')
      const response = await fetch(`http://localhost:8080/api/v1/admin/users/${username}/${endpoint}`, {
        method: 'POST',
        headers: {
          'X-Admin-Token': adminToken || '',
        },
      })

      if (response.ok) {
        fetchAdmins()
      } else {
        setError(`Failed to ${endpoint} admin`)
      }
    } catch (err) {
      setError('Network error. Please try again.')
    }
  }

  const formatDate = (dateString: string | null) => {
    if (!dateString) return 'Never'
    return new Date(dateString).toLocaleString()
  }

  return (
    <div className="flex-1 p-8">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="p-2 rounded-lg bg-primary/10">
                <Users className="w-6 h-6 text-primary" />
              </div>
              <h1 className="text-3xl font-bold">Manage Admins</h1>
            </div>
            <p className="text-muted-foreground">
              Create and manage administrator accounts
            </p>
          </div>
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="px-6 py-3 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-all flex items-center gap-2"
          >
            <Plus className="w-5 h-5" />
            Create Admin
          </button>
        </div>

        {/* Error Message */}
        {error && (
          <div className="flex items-center gap-3 p-4 rounded-lg bg-red-500/10 border border-red-500/20 mb-6">
            <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0" />
            <p className="text-sm text-red-500">{error}</p>
          </div>
        )}

        {/* Create Form */}
        {showCreateForm && (
          <div className="glass-card p-6 mb-6">
            <h2 className="text-xl font-bold mb-4">Create New Admin</h2>
            <form onSubmit={handleCreateAdmin} className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium mb-2">Username*</label>
                <input
                  type="text"
                  required
                  value={formData.username}
                  onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                  className="w-full px-4 py-2 rounded-lg bg-background/50 border border-border focus:border-primary focus:ring-2 focus:ring-primary/20 outline-none transition-all"
                  placeholder="username"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">Password*</label>
                <input
                  type="password"
                  required
                  value={formData.password}
                  onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                  className="w-full px-4 py-2 rounded-lg bg-background/50 border border-border focus:border-primary focus:ring-2 focus:ring-primary/20 outline-none transition-all"
                  placeholder="********"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">Email</label>
                <input
                  type="email"
                  value={formData.email}
                  onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                  className="w-full px-4 py-2 rounded-lg bg-background/50 border border-border focus:border-primary focus:ring-2 focus:ring-primary/20 outline-none transition-all"
                  placeholder="admin@example.com"
                />
              </div>
              <div>
                <label className="block text-sm font-medium mb-2">Full Name</label>
                <input
                  type="text"
                  value={formData.full_name}
                  onChange={(e) => setFormData({ ...formData, full_name: e.target.value })}
                  className="w-full px-4 py-2 rounded-lg bg-background/50 border border-border focus:border-primary focus:ring-2 focus:ring-primary/20 outline-none transition-all"
                  placeholder="John Doe"
                />
              </div>
              <div className="col-span-2 flex gap-4">
                <button
                  type="submit"
                  disabled={createLoading}
                  className="px-6 py-2 rounded-lg bg-primary text-primary-foreground font-medium hover:opacity-90 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {createLoading ? 'Creating...' : 'Create Admin'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreateForm(false)
                    setError('')
                  }}
                  className="px-6 py-2 rounded-lg border border-border hover:bg-accent transition-all"
                >
                  Cancel
                </button>
              </div>
            </form>
          </div>
        )}

        {/* Admin List */}
        {loading ? (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="w-8 h-8 animate-spin text-primary" />
          </div>
        ) : (
          <div className="glass-card overflow-hidden">
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b border-border">
                    <th className="text-left p-4 font-medium">Username</th>
                    <th className="text-left p-4 font-medium">Full Name</th>
                    <th className="text-left p-4 font-medium">Email</th>
                    <th className="text-left p-4 font-medium">Created</th>
                    <th className="text-left p-4 font-medium">Last Login</th>
                    <th className="text-left p-4 font-medium">Status</th>
                    <th className="text-left p-4 font-medium">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {admins.map((admin) => (
                    <tr key={admin.id} className="border-b border-border/50 hover:bg-accent/10">
                      <td className="p-4 font-medium">{admin.username}</td>
                      <td className="p-4">{admin.full_name || '-'}</td>
                      <td className="p-4">{admin.email || '-'}</td>
                      <td className="p-4 text-sm text-muted-foreground">
                        {formatDate(admin.created_at)}
                      </td>
                      <td className="p-4 text-sm text-muted-foreground">
                        {formatDate(admin.last_login_at)}
                      </td>
                      <td className="p-4">
                        <span
                          className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${
                            admin.is_active
                              ? 'bg-green-500/10 text-green-500'
                              : 'bg-gray-500/10 text-gray-500'
                          }`}
                        >
                          {admin.is_active ? (
                            <>
                              <UserCheck className="w-3 h-3" />
                              Active
                            </>
                          ) : (
                            <>
                              <UserX className="w-3 h-3" />
                              Inactive
                            </>
                          )}
                        </span>
                      </td>
                      <td className="p-4">
                        <button
                          onClick={() => handleToggleStatus(admin.username, admin.is_active)}
                          className={`px-3 py-1.5 rounded text-sm font-medium transition-all ${
                            admin.is_active
                              ? 'text-red-500 hover:bg-red-500/10'
                              : 'text-green-500 hover:bg-green-500/10'
                          }`}
                        >
                          {admin.is_active ? 'Deactivate' : 'Activate'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </div>
    </div>
  )
}

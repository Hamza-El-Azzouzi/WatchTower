'use client'

import { useState } from 'react'
import { useRouter } from 'next/navigation'
import { Send, Users, Server, Building2, Mail, Phone, MessageSquare, CheckCircle, AlertCircle, Loader2 } from 'lucide-react'
import Link from 'next/link'

interface RequestFormData {
  company_name: string
  contact_name: string
  email: string
  phone: string
  agents_requested: number
  use_case: string
  message: string
}

const AGENT_TIERS = [
  { value: 5, label: '5 Agents', description: 'Starter - Perfect for small teams', price: 'Free' },
  { value: 25, label: '25 Agents', description: 'Growth - For growing infrastructure', price: '$49/mo' },
  { value: 50, label: '50 Agents', description: 'Professional - Medium-sized deployments', price: '$99/mo' },
  { value: 100, label: '100 Agents', description: 'Business - Large infrastructure', price: '$199/mo' },
  { value: 250, label: '250 Agents', description: 'Enterprise - Enterprise scale', price: '$399/mo' },
  { value: 500, label: '500+ Agents', description: 'Custom - Unlimited scaling', price: 'Custom' },
]

const USE_CASES = [
  'Server Monitoring',
  'Database Monitoring',
  'Container/Kubernetes',
  'Cloud Infrastructure (AWS/GCP/Azure)',
  'Microservices',
  'CI/CD Pipelines',
  'Development/Testing',
  'Production Systems',
  'Hybrid Cloud',
  'Other',
]

export default function RequestAgentsPage() {
  const router = useRouter()
  const [formData, setFormData] = useState<RequestFormData>({
    company_name: '',
    contact_name: '',
    email: '',
    phone: '',
    agents_requested: 25,
    use_case: '',
    message: '',
  })
  const [loading, setLoading] = useState(false)
  const [success, setSuccess] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setError(null)
    setLoading(true)

    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'
      const response = await fetch(`${apiUrl}/api/v1/requests/agents`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(formData),
      })

      if (response.ok) {
        setSuccess(true)
      } else {
        const errorData = await response.json().catch(() => ({}))
        setError(errorData.error || 'Failed to submit request. Please try again.')
      }
    } catch (err) {
      setError('Connection error. Please try again later.')
    } finally {
      setLoading(false)
    }
  }

  const handleChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
    const { name, value } = e.target
    setFormData(prev => ({
      ...prev,
      [name]: name === 'agents_requested' ? parseInt(value) : value,
    }))
  }

  if (success) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 flex items-center justify-center p-4">
        <div className="max-w-md w-full text-center">
          <div className="bg-gray-800/50 backdrop-blur-sm rounded-2xl p-8 border border-gray-700/50 shadow-xl">
            <div className="inline-flex items-center justify-center w-20 h-20 bg-green-500/20 rounded-full mb-6 border border-green-500/30">
              <CheckCircle className="w-10 h-10 text-green-400" />
            </div>
            <h1 className="text-2xl font-bold text-white mb-4">Request Submitted!</h1>
            <p className="text-gray-400 mb-6">
              Thank you for your interest! Our team will review your request and get back to you within 24-48 hours.
            </p>
            <div className="bg-gray-700/30 rounded-lg p-4 mb-6 text-left">
              <p className="text-sm text-gray-300">
                <span className="text-gray-500">Company:</span> {formData.company_name}
              </p>
              <p className="text-sm text-gray-300">
                <span className="text-gray-500">Agents Requested:</span> {formData.agents_requested}
              </p>
              <p className="text-sm text-gray-300">
                <span className="text-gray-500">Email:</span> {formData.email}
              </p>
            </div>
            <div className="space-y-3">
              <Link
                href="/"
                className="block w-full bg-blue-600 hover:bg-blue-700 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
              >
                Go to Dashboard
              </Link>
              <Link
                href="/login"
                className="block w-full bg-gray-700 hover:bg-gray-600 text-white font-semibold py-3 px-4 rounded-lg transition-colors"
              >
                Login with API Key
              </Link>
            </div>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900 py-12 px-4">
      <div className="max-w-2xl mx-auto">
        {/* Header */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-500/20 rounded-2xl mb-4 border border-blue-500/30">
            <Server className="w-8 h-8 text-blue-400" />
          </div>
          <h1 className="text-3xl font-bold text-white mb-2">Request Agent Quota</h1>
          <p className="text-gray-400 max-w-md mx-auto">
            Tell us about your monitoring needs and we&apos;ll set up your account with the right number of agents.
          </p>
        </div>

        {/* Form Card */}
        <div className="bg-gray-800/50 backdrop-blur-sm rounded-2xl p-8 border border-gray-700/50 shadow-xl">
          {error && (
            <div className="mb-6 p-4 bg-red-500/10 border border-red-500/30 rounded-lg flex items-center gap-3 text-red-400">
              <AlertCircle className="w-5 h-5 flex-shrink-0" />
              <span className="text-sm">{error}</span>
            </div>
          )}

          <form onSubmit={handleSubmit} className="space-y-6">
            {/* Company Info Section */}
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                <Building2 className="w-5 h-5 text-blue-400" />
                Company Information
              </h3>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label htmlFor="company_name" className="block text-sm font-medium text-gray-300 mb-2">
                    Company Name *
                  </label>
                  <input
                    type="text"
                    id="company_name"
                    name="company_name"
                    value={formData.company_name}
                    onChange={handleChange}
                    required
                    className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                    placeholder="Acme Corporation"
                  />
                </div>
                <div>
                  <label htmlFor="contact_name" className="block text-sm font-medium text-gray-300 mb-2">
                    Contact Name *
                  </label>
                  <input
                    type="text"
                    id="contact_name"
                    name="contact_name"
                    value={formData.contact_name}
                    onChange={handleChange}
                    required
                    className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                    placeholder="John Doe"
                  />
                </div>
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label htmlFor="email" className="block text-sm font-medium text-gray-300 mb-2">
                    <Mail className="w-4 h-4 inline mr-1" />
                    Email Address *
                  </label>
                  <input
                    type="email"
                    id="email"
                    name="email"
                    value={formData.email}
                    onChange={handleChange}
                    required
                    className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                    placeholder="john@company.com"
                  />
                </div>
                <div>
                  <label htmlFor="phone" className="block text-sm font-medium text-gray-300 mb-2">
                    <Phone className="w-4 h-4 inline mr-1" />
                    Phone (Optional)
                  </label>
                  <input
                    type="tel"
                    id="phone"
                    name="phone"
                    value={formData.phone}
                    onChange={handleChange}
                    className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                    placeholder="+1 (555) 123-4567"
                  />
                </div>
              </div>
            </div>

            {/* Agent Request Section */}
            <div className="space-y-4 pt-4 border-t border-gray-700">
              <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                <Users className="w-5 h-5 text-blue-400" />
                Agent Requirements
              </h3>

              <div>
                <label htmlFor="agents_requested" className="block text-sm font-medium text-gray-300 mb-2">
                  Number of Agents Needed *
                </label>
                <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
                  {AGENT_TIERS.map((tier) => (
                    <button
                      key={tier.value}
                      type="button"
                      onClick={() => setFormData(prev => ({ ...prev, agents_requested: tier.value }))}
                      className={`p-4 rounded-lg border text-left transition-all ${
                        formData.agents_requested === tier.value
                          ? 'bg-blue-600/20 border-blue-500 ring-2 ring-blue-500'
                          : 'bg-gray-700/30 border-gray-600 hover:border-gray-500'
                      }`}
                    >
                      <div className="font-bold text-white">{tier.label}</div>
                      <div className="text-xs text-gray-400 mt-1">{tier.description}</div>
                      <div className="text-sm font-semibold text-blue-400 mt-2">{tier.price}</div>
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <label htmlFor="use_case" className="block text-sm font-medium text-gray-300 mb-2">
                  Primary Use Case *
                </label>
                <select
                  id="use_case"
                  name="use_case"
                  value={formData.use_case}
                  onChange={handleChange}
                  required
                  className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                >
                  <option value="">Select a use case...</option>
                  {USE_CASES.map((useCase) => (
                    <option key={useCase} value={useCase}>{useCase}</option>
                  ))}
                </select>
              </div>
            </div>

            {/* Message Section */}
            <div className="space-y-4 pt-4 border-t border-gray-700">
              <h3 className="text-lg font-semibold text-white flex items-center gap-2">
                <MessageSquare className="w-5 h-5 text-blue-400" />
                Additional Information
              </h3>
              
              <div>
                <label htmlFor="message" className="block text-sm font-medium text-gray-300 mb-2">
                  Tell us more about your monitoring needs (Optional)
                </label>
                <textarea
                  id="message"
                  name="message"
                  value={formData.message}
                  onChange={handleChange}
                  rows={4}
                  className="w-full bg-gray-700/50 border border-gray-600 rounded-lg px-4 py-3 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all resize-none"
                  placeholder="Describe your infrastructure, specific requirements, or any questions you have..."
                />
              </div>
            </div>

            {/* Submit Button */}
            <div className="pt-4">
              <button
                type="submit"
                disabled={loading}
                className="w-full bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 disabled:from-gray-600 disabled:to-gray-700 text-white font-bold py-4 px-6 rounded-lg transition-all flex items-center justify-center gap-2 shadow-lg shadow-blue-500/25"
              >
                {loading ? (
                  <>
                    <Loader2 className="w-5 h-5 animate-spin" />
                    Submitting Request...
                  </>
                ) : (
                  <>
                    <Send className="w-5 h-5" />
                    Submit Request
                  </>
                )}
              </button>
            </div>
          </form>

          {/* Footer Links */}
          <div className="mt-6 pt-6 border-t border-gray-700 text-center space-y-2">
            <p className="text-gray-400 text-sm">
              Already have an API key?{' '}
              <Link href="/login" className="text-blue-400 hover:text-blue-300 font-medium">
                Sign in here
              </Link>
            </p>
            <p className="text-gray-500 text-xs">
              Need admin access?{' '}
              <Link href="/admin-login" className="text-gray-400 hover:text-gray-300">
                Admin Login
              </Link>
            </p>
          </div>
        </div>

        {/* Trust Indicators */}
        <div className="mt-8 text-center">
          <p className="text-gray-500 text-sm mb-4">Trusted by DevOps teams worldwide</p>
          <div className="flex justify-center gap-8 text-gray-600">
            <div className="text-center">
              <div className="text-2xl font-bold text-white">10K+</div>
              <div className="text-xs">Agents Deployed</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">99.9%</div>
              <div className="text-xs">Uptime SLA</div>
            </div>
            <div className="text-center">
              <div className="text-2xl font-bold text-white">&lt;50ms</div>
              <div className="text-xs">Latency</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

/** @type {import('next').NextConfig} */
const isDevelopment = process.env.NODE_ENV !== 'production'

function cspOrigin(value, fallback) {
  try {
    const parsed = new URL(value || fallback)
    return `${parsed.protocol}//${parsed.host}`
  } catch {
    return fallback
  }
}

const configuredConnections = [
  cspOrigin(process.env.NEXT_PUBLIC_API_URL, 'http://localhost:8080'),
  cspOrigin(process.env.NEXT_PUBLIC_WS_URL, 'ws://localhost:8080'),
].filter(Boolean).join(' ')

const contentSecurityPolicy = [
  "default-src 'self'",
  `script-src 'self' 'unsafe-inline'${isDevelopment ? " 'unsafe-eval'" : ''}`,
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self' data:",
  `connect-src 'self' ${configuredConnections}`.trim(),
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'self'",
  "frame-ancestors 'none'",
].join('; ')

const securityHeaders = [
  { key: 'Content-Security-Policy', value: contentSecurityPolicy },
  { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
  { key: 'X-Content-Type-Options', value: 'nosniff' },
  { key: 'X-Frame-Options', value: 'DENY' },
  { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=()' },
]

const nextConfig = {
  // Disable React Strict Mode to prevent double mount/unmount of effects
  reactStrictMode: false,
  images: {
    unoptimized: true,
  },
  output: 'standalone',
  // Use the stable compiler API path. The experimental CLI path can emit
  // non-JSON wrapper output in confined Node installations and break builds.
  experimental: {
    useTypeScriptCli: false,
  },
  async headers() {
    return [{ source: '/(.*)', headers: securityHeaders }]
  },
}

export default nextConfig

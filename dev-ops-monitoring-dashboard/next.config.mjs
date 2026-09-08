/** @type {import('next').NextConfig} */
const nextConfig = {
  // Disable React Strict Mode to prevent double mount/unmount of effects
  reactStrictMode: false,
  images: {
    unoptimized: true,
  },
  output: 'standalone',
}

export default nextConfig

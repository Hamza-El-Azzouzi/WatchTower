# Phase 4: Web Dashboard - Implementation Guide

## Overview
Phase 4 delivers a professional, real-time web dashboard for visualizing monitoring data. Built with Next.js 16, React 19, and Recharts, the dashboard provides comprehensive system health insights, historical trends, alert thresholds, and multi-server comparisons.

## Objectives
- ✅ Create a modern, responsive web dashboard using Next.js
- ✅ Display real-time metrics from all registered agents
- ✅ Visualize historical data with interactive charts
- ✅ Implement alert threshold visualization with colored zones
- ✅ Add system health overview with multiple chart types
- ✅ Support server comparison and resource distribution views
- ✅ Enable auto-refresh for real-time monitoring

## Technology Stack

### Frontend Framework
- **Next.js 16.0.10** - React framework with server-side rendering
- **React 19.2.0** - Latest React with modern hooks
- **TypeScript 5** - Type-safe development
- **Tailwind CSS 4** - Utility-first CSS framework

### UI Components
- **Radix UI** - Accessible component primitives
- **Lucide React** - Beautiful icon library
- **Recharts 2.15.4** - Powerful charting library
- **Sonner** - Toast notifications

### Features
- **Glass Morphism** - Modern frosted glass UI design
- **Dark Theme** - Professional dark color scheme
- **Responsive Design** - Mobile-first approach
- **Real-time Updates** - 10-second polling interval

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 Next.js Dashboard (Port 3000)           │
│                                                          │
│  ┌────────────────┐         ┌────────────────┐         │
│  │  Overview Page │         │ Server Detail  │         │
│  │                │         │     Page       │         │
│  │  - Stats Cards │         │  - Metrics     │         │
│  │  - Health View │         │  - Thresholds  │         │
│  │  - Server Grid │         │  - Charts      │         │
│  └────────┬───────┘         └───────┬────────┘         │
│           │                         │                   │
│           └─────────┬───────────────┘                   │
│                     │                                   │
│                     ▼                                   │
│            ┌─────────────────┐                          │
│            │   API Client    │                          │
│            │  (lib/api.ts)   │                          │
│            └────────┬────────┘                          │
└─────────────────────┼──────────────────────────────────┘
                      │
                      │ HTTP GET (10s polling)
                      ▼
        ┌──────────────────────────────┐
        │   Central Server (Port 8080)│
        │                              │
        │  REST API Endpoints:         │
        │  - GET /api/v1/agents        │
        │  - GET /api/v1/metrics       │
        │  - GET /api/v1/metrics/latest│
        │  - GET /api/v1/stats         │
        │  - GET /health               │
        └──────────────────────────────┘
```

## Project Structure

```
dev-ops-monitoring-dashboard/
├── app/
│   ├── page.tsx                 # Overview page (main dashboard)
│   ├── server/
│   │   └── [agentId]/
│   │       └── page.tsx         # Server detail page
│   ├── layout.tsx               # Root layout with theme provider
│   └── globals.css              # Global styles with animations
├── components/
│   ├── AlertThresholdChart.tsx  # 🆕 Alert threshold visualization
│   ├── SystemHealthOverview.tsx # 🆕 Health overview with charts
│   ├── ChartsSection.tsx        # Updated historical charts
│   ├── ServerCard.tsx           # Server card with metrics
│   ├── StatsCards.tsx           # Summary statistics cards
│   ├── MetricsSection.tsx       # Current metrics display
│   ├── PageHeader.tsx           # Dashboard header
│   ├── HeroBanner.tsx           # Hero section
│   ├── StatusBadge.tsx          # Agent status indicator
│   └── ui/                      # Radix UI components
├── lib/
│   ├── api.ts                   # API client functions
│   └── metrics-utils.ts         # Utility functions
├── types/
│   └── index.ts                 # TypeScript type definitions
├── package.json                 # Dependencies
└── tsconfig.json                # TypeScript configuration
```

## Key Features Implemented

### 1. System Health Overview (New!)

**Component: `SystemHealthOverview.tsx`**

Displays aggregate health metrics across all servers with three visualization types:

#### Overall Health Score (Radial Chart)
- Calculates health score based on CPU, memory, and disk usage
- Color-coded: Green (>80%), Amber (60-80%), Red (<60%)
- Real-time updates every 10 seconds

```typescript
const getHealthScore = (): number => {
  const avgCpu = agents.reduce((sum, agent) => 
    sum + getMetricValue(agent.id, 'cpu_usage'), 0) / agents.length;
  const avgMemory = agents.reduce((sum, agent) => 
    sum + getMetricValue(agent.id, 'memory_usage'), 0) / agents.length;
  const avgDisk = agents.reduce((sum, agent) => 
    sum + getMetricValue(agent.id, 'disk_usage'), 0) / agents.length;
  
  // 100 is perfect, lower is worse
  const cpuScore = Math.max(0, 100 - avgCpu);
  const memoryScore = Math.max(0, 100 - avgMemory);
  const diskScore = Math.max(0, 100 - avgDisk);
  
  return ((cpuScore + memoryScore + diskScore) / 3);
};
```

#### Resource Distribution (Pie Chart)
- Shows average CPU, memory, and disk usage across all servers
- Color-coded segments: Blue (CPU), Green (Memory), Amber (Disk)
- Interactive tooltips with detailed percentages

#### Server Comparison (Bar Chart)
- Side-by-side comparison of all servers
- Three metrics per server: CPU, Memory, Disk
- Grouped bars with clear legends
- Truncated server names for readability

### 2. Alert Threshold Visualization (New!)

**Component: `AlertThresholdChart.tsx`**

Advanced chart showing metrics with warning and critical thresholds:

#### Features:
- **Colored Zones**: 
  - Green zone (0-70%): Normal operation
  - Amber zone (70-90%): Warning level
  - Red zone (90-100%): Critical level

- **Reference Lines**: Dashed lines at threshold boundaries

- **Status Indicators**:
  - Real-time status badge (Normal/Warning/Critical)
  - Color-coded current value display
  - Alert messages when thresholds exceeded

- **Configurable Thresholds**:
  ```typescript
  <AlertThresholdChart
    agentId={agentId}
    metric="cpu_usage"
    title="CPU Usage with Thresholds"
    warningThreshold={70}
    criticalThreshold={90}
  />
  ```

### 3. Enhanced Historical Charts

**Component: `ChartsSection.tsx`**

Updated to work with the correct metric names from the agent:

#### Metrics Displayed:
1. **CPU Usage** - Line chart with percentage (0-100%)
2. **Memory Usage** - Area chart with percentage (0-100%)
3. **Disk Usage** - Area chart with percentage (0-100%)
4. **Network Traffic** - Dual line chart showing RX and TX in MB

#### Time Range Selector:
- 1 Hour (360 data points)
- 6 Hours (2,160 data points)
- 24 Hours (8,640 data points)
- 7 Days (60,480 data points)

### 4. Real-Time Updates

All components implement automatic data refresh:

```typescript
useEffect(() => {
  fetchData();
  
  // Poll every 10 seconds
  const interval = setInterval(fetchData, 10000);
  return () => clearInterval(interval);
}, [agentId]);
```

### 5. Responsive Design

- **Mobile**: Single column layout
- **Tablet**: 2-column grid
- **Desktop**: 3-column grid
- **Large Desktop**: Full-width charts

## API Integration

### API Client (`lib/api.ts`)

```typescript
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

// Fetch all registered agents
export async function getAgents(): Promise<Agent[]> {
  const response = await fetch(`${API_BASE_URL}/api/v1/agents`);
  return response.json();
}

// Get latest metrics for a specific agent
export async function getLatestMetrics(agentId: string): Promise<LatestMetrics> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/metrics/latest?agent_id=${agentId}`
  );
  return response.json();
}

// Get historical data for a specific metric
export async function getHistoricalMetrics(
  agentId: string,
  metric: string,
  limit: number = 100
): Promise<HistoricalMetrics> {
  const response = await fetch(
    `${API_BASE_URL}/api/v1/metrics?agent_id=${agentId}&metric=${metric}&limit=${limit}`
  );
  return response.json();
}
```

### Metric Name Mapping

The agent sends metrics with these names:
- `cpu_usage` - CPU percentage (0-100)
- `memory_usage` - Memory percentage (0-100)
- `disk_usage` - Disk percentage (0-100)
- `network_rx_bytes` - Received bytes (cumulative)
- `network_tx_bytes` - Transmitted bytes (cumulative)

## Installation & Setup

### 1. Install Dependencies

```bash
cd dev-ops-monitoring-dashboard
npm install
```

**Key Dependencies:**
```json
{
  "next": "16.0.10",
  "react": "19.2.0",
  "recharts": "2.15.4",
  "lucide-react": "^0.454.0",
  "tailwindcss": "^4.1.9",
  "@radix-ui/react-*": "Latest"
}
```

### 2. Environment Configuration

Create `.env.local` (optional):
```env
NEXT_PUBLIC_API_URL=http://localhost:8080
```

### 3. Start Development Server

```bash
npm run dev
```

Dashboard will be available at: **http://localhost:3000**

### 4. Build for Production

```bash
npm run build
npm start
```

## Pages & Routes

### Overview Page (`/`)

**Features:**
- Hero banner with system introduction
- Summary statistics cards (Total Servers, Healthy, Warning, Critical)
- System Health Overview section with 3 chart types
- Server cards grid with real-time metrics
- Auto-refresh indicator
- Last updated timestamp

**Components Used:**
- `PageHeader`
- `HeroBanner`
- `StatsCards`
- `SystemHealthOverview` (new!)
- `ServerCard` (grid)

### Server Detail Page (`/server/[agentId]`)

**Features:**
- Back to overview navigation
- Server name and status
- Current metrics section
- Alert Thresholds section (new!) with 3 charts
- Historical Charts section with 4 charts
- Time range selector (1h, 6h, 24h, 7d)

**Components Used:**
- `StatusBadge`
- `MetricsSection`
- `AlertThresholdChart` (new!) x3
- `ChartsSection`

## Design System

### Color Palette

```css
/* Background Colors */
--background: #0f0f0f;
--card: #1a1a1a;

/* Accent Colors */
--primary: #6366f1;    /* Indigo */
--secondary: #3b82f6;  /* Blue */
--accent: #06b6d4;     /* Cyan */

/* Status Colors */
--success: #10b981;    /* Green */
--warning: #f59e0b;    /* Amber */
--danger: #ef4444;     /* Red */

/* Text Colors */
--foreground: #f5f5f5;
--muted: #9ca3af;
```

### Glass Morphism Effect

```css
.glass-morphism {
  background: rgba(26, 26, 26, 0.8);
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
}
```

### Animations

```css
@keyframes slide-up {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@keyframes pulse-soft {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}

.animate-slide-up {
  animation: slide-up 0.5s ease-out;
}
```

## Testing the Dashboard

### Terminal 1: Start Server
```bash
cd server
cargo run --release

# Expected output:
# INFO Starting server on 0.0.0.0:8080
# INFO API Endpoints:
#   POST   /api/v1/metrics         - Ingest metrics from agents
#   GET    /api/v1/metrics         - Query historical metrics
#   ...
```

### Terminal 2: Start Agent
```bash
cd agent
cargo run -- -c agent.toml

# Expected output:
# INFO Server integration enabled - sending metrics to http://localhost:8080
# --------------------------- Monitoring Agent Started ---------------------------
# Agent: dev-server-01 | Interval: 10s | Server: http://localhost:8080
# 
# [2026-01-28 08:47:24] CPU: 6.9%, Memory: 9.79 GB/15.30 GB (64.0%), ...
```

### Terminal 3: Start Dashboard
```bash
cd dev-ops-monitoring-dashboard
npm run dev

# Expected output:
#    ▲ Next.js 16.0.10 (Turbopack)
#    - Local:         http://localhost:3000
#    - Network:       http://10.1.5.1:3000
#  ✓ Ready in 341ms
```

### Verification Steps

1. **Open Dashboard**: Navigate to http://localhost:3000

2. **Check Overview Page**:
   - ✅ Stats cards showing "1 Total Servers", "1 Healthy"
   - ✅ System Health Overview with radial chart, pie chart, and bar chart
   - ✅ Server card for "dev-server-01" with current metrics
   - ✅ Auto-refresh indicator blinking

3. **Click Server Card**: Navigate to detail page

4. **Check Detail Page**:
   - ✅ Current metrics section with latest values
   - ✅ Alert Thresholds section with 3 threshold charts
   - ✅ CPU/Memory/Disk showing colored zones
   - ✅ Historical Charts section with 4 time-series charts
   - ✅ Time range selector working (1h, 6h, 24h, 7d)

5. **Test Real-Time Updates**:
   - Wait 10 seconds
   - Metrics should update automatically
   - Charts should show new data points
   - Last updated timestamp should change

## Performance Optimization

### Data Fetching
- **Parallel Requests**: All metrics fetched concurrently with `Promise.all`
- **Revalidation**: 10-second cache for server-side fetching
- **Client Polling**: 10-second intervals for real-time updates

### Chart Performance
- **Animation Disabled**: `isAnimationActive={false}` on all charts
- **Dot Rendering**: `dot={false}` on line charts to reduce DOM nodes
- **Data Limiting**: Configurable limits for historical data

### Bundle Size
- **Code Splitting**: Automatic per-page splitting by Next.js
- **Tree Shaking**: Unused Radix UI components excluded
- **Dynamic Imports**: Large components lazy-loaded

## Troubleshooting

### Issue: "Failed to load servers"

**Symptoms:**
```
Connection Error
Failed to load servers. Make sure the API is running at http://localhost:8080
```

**Solutions:**
1. Check server is running: `curl http://localhost:8080/health`
2. Verify CORS is enabled in server
3. Check firewall rules

### Issue: No data in charts

**Symptoms:**
- Charts show "No data available"
- Metrics section is empty

**Solutions:**
1. Ensure agent is sending metrics:
   ```bash
   curl "http://localhost:8080/api/v1/agents"
   ```
2. Check agent logs for send errors
3. Verify metric names match (cpu_usage not cpu_percent)

### Issue: Charts not updating

**Symptoms:**
- Old data shown
- No auto-refresh

**Solutions:**
1. Check browser console for errors
2. Verify polling interval is running
3. Clear browser cache
4. Check API response times

### Issue: Threshold zones not showing

**Symptoms:**
- AlertThresholdChart shows no colored zones

**Solutions:**
1. Verify ReferenceArea component imports
2. Check threshold prop values (0-100)
3. Ensure data has correct range

## Enhancement Opportunities

### Future Improvements

1. **WebSocket Support**
   - Replace polling with WebSocket for true real-time updates
   - Reduce server load and network traffic
   - Instant metric updates

2. **Advanced Filtering**
   - Filter servers by status
   - Search servers by name
   - Group servers by tags

3. **Custom Dashboards**
   - User-defined layouts
   - Draggable widgets
   - Saved preferences

4. **Alerting**
   - Email notifications on threshold breach
   - Slack/Discord integrations
   - Alert history log

5. **Export Features**
   - Download charts as images
   - Export data as CSV
   - PDF reports

6. **Historical Analysis**
   - Zoom and pan on charts
   - Date range picker
   - Comparison between time periods

7. **Multi-Tenant Support**
   - Authentication (NextAuth.js)
   - Role-based access control
   - Organization management

## Component Reference

### AlertThresholdChart

```typescript
interface AlertThresholdChartProps {
  agentId: string;
  metric: 'cpu_usage' | 'memory_usage' | 'disk_usage';
  title: string;
  warningThreshold?: number;    // Default: 70
  criticalThreshold?: number;   // Default: 90
  limit?: number;               // Default: 100
}
```

**Features:**
- Composed chart with area fill and line
- Reference areas for Normal/Warning/Critical zones
- Reference lines with labels
- Real-time status indicator
- Alert messages when thresholds exceeded

### SystemHealthOverview

```typescript
interface SystemHealthOverviewProps {
  agents: Agent[];
}
```

**Features:**
- Calculates aggregate health across all agents
- Three chart types: Radial, Pie, Bar
- Real-time metric fetching for all agents
- Auto-refresh every 10 seconds

### ChartsSection

```typescript
interface ChartsSectionProps {
  agentId: string;
}
```

**Features:**
- Four chart types: CPU (line), Memory (area), Disk (area), Network (dual line)
- Time range selector: 1h, 6h, 24h, 7d
- Automatic data point limiting
- Skeleton loaders during fetch

## Keyboard Shortcuts

- **Escape**: Close modals (if any)
- **Ctrl + R**: Refresh page manually
- **Ctrl + /**: Focus search (when implemented)

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

## Accessibility

- Semantic HTML structure
- ARIA labels on interactive elements
- Keyboard navigation support
- Color contrast ratios meet WCAG 2.1 AA
- Screen reader friendly

## Production Deployment

### Environment Variables

```env
NEXT_PUBLIC_API_URL=https://api.monitoring.example.com
NODE_ENV=production
```

### Build Command

```bash
npm run build
```

### Start Production Server

```bash
npm start
```

### Docker Deployment

```dockerfile
FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .
RUN npm run build

EXPOSE 3000

CMD ["npm", "start"]
```

### Build Docker Image

```bash
docker build -t monitoring-dashboard .
docker run -p 3000:3000 -e NEXT_PUBLIC_API_URL=http://api:8080 monitoring-dashboard
```

## Next Steps

Phase 4 completes the core monitoring dashboard. The system now has:
- ✅ Agents collecting metrics locally
- ✅ Central server with REST API and storage
- ✅ Agent-to-server communication with retry logic
- ✅ Beautiful web dashboard with real-time visualizations

**Phase 5** will add:
- Application log collection and aggregation
- Log search and filtering
- Log correlation with metrics

**Phase 6** will implement:
- Alert engine with threshold rules
- Notification channels (email, Slack, etc.)
- Alert history and acknowledgment

**Phase 7** will focus on:
- Production hardening
- Security enhancements
- Performance optimization
- Documentation and deployment guides

---

## Summary of Phase 4 Achievements

✅ **Modern Dashboard**: Next.js 16 + React 19 + TypeScript
✅ **Rich Visualizations**: 7+ chart types (Line, Area, Bar, Pie, Radial)
✅ **Real-Time Updates**: 10-second polling across all components
✅ **Alert Thresholds**: Visual threshold zones with status indicators
✅ **Health Overview**: System-wide health score and resource distribution
✅ **Server Comparison**: Side-by-side metric comparison
✅ **Responsive Design**: Mobile-first with glass morphism
✅ **Production Ready**: Build and deployment configurations

**Phase 4 Status:** ✅ **COMPLETE**

The dashboard successfully visualizes all metrics from the monitoring agents with professional charts, real-time updates, and comprehensive health insights!

# V0 Prompt for DevOps Monitoring System Dashboard

## Project Overview
Create a modern, real-time monitoring dashboard for a DevOps infrastructure monitoring system. This dashboard displays system metrics (CPU, memory, disk, network) from multiple servers, with live updates, historical charts, and agent health status.

---

## Tech Stack Requirements
- **Framework**: Next.js 14+ (App Router)
- **Styling**: Tailwind CSS
- **Charts**: Recharts or Chart.js
- **UI Components**: shadcn/ui
- **Icons**: Lucide React
- **Real-time Updates**: Polling (fetch every 10 seconds)

---

## API Endpoints Available

### Base URL: `http://localhost:8080`

**1. GET /api/v1/agents**
- Returns list of all monitored servers
```json
[
  {
    "id": "web-server-01",
    "name": "web-server-01",
    "last_seen": "2026-01-27T10:30:55Z",
    "status": "Healthy" // or "Degraded" or "Unreachable"
  }
]
```

**2. GET /api/v1/metrics/latest?agent_id={agent_id}**
- Returns current values for all metrics of a specific agent
```json
{
  "agent_id": "web-server-01",
  "metrics": [
    {
      "name": "cpu_percent",
      "value": 45.2,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "memory_used_bytes",
      "value": 4294967296,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "memory_total_bytes",
      "value": 8589934592,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "disk_used_bytes",
      "value": 107374182400,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "disk_total_bytes",
      "value": 536870912000,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "network_rx_bytes",
      "value": 1048576000,
      "timestamp": "2026-01-27T10:30:55Z"
    },
    {
      "name": "network_tx_bytes",
      "value": 524288000,
      "timestamp": "2026-01-27T10:30:55Z"
    }
  ]
}
```

**3. GET /api/v1/metrics?agent_id={agent_id}&metric={metric_name}&limit={limit}**
- Returns historical data points for a specific metric
```json
{
  "agent_id": "web-server-01",
  "metric": "cpu_percent",
  "count": 100,
  "data_points": [
    {
      "timestamp": "2026-01-27T10:30:00Z",
      "value": 42.5
    },
    {
      "timestamp": "2026-01-27T10:30:10Z",
      "value": 45.2
    }
    // ... more data points
  ]
}
```

**4. GET /api/v1/stats**
- Returns overall storage statistics
```json
{
  "total_agents": 5,
  "total_metrics": 35,
  "total_data_points": 10500
}
```

---

## Dashboard Layout Requirements

### Page 1: Overview Dashboard (/)

**Header Section:**
- Title: "DevOps Monitoring System"
- Real-time clock showing current time
- Auto-refresh indicator (updates every 10 seconds)
- Overall statistics cards in a row:
  - Total Servers (with icon)
  - Healthy Servers (green badge)
  - Degraded Servers (yellow badge)
  - Unreachable Servers (red badge)

**Server List Section:**
- Grid of cards, one for each agent/server
- Each card shows:
  - Server name (large, bold)
  - Status badge (green for Healthy, yellow for Degraded, red for Unreachable)
  - Last seen time (relative: "2 minutes ago")
  - Mini stats preview:
    - CPU: XX.X%
    - Memory: X.X GB / X.X GB (XX%)
    - Disk: XX GB / XXX GB (XX%)
  - "View Details" button
- Cards should have hover effect and be clickable
- Use color coding: green border for healthy, yellow for degraded, red for unreachable

**Footer:**
- Last updated timestamp
- Link to server stats

---

### Page 2: Server Detail View (/server/[agentId])

**Header:**
- Back button to overview
- Server name and status badge
- Last seen timestamp

**Current Metrics Section (Top Row of Cards):**
Four large metric cards showing current values:

1. **CPU Usage Card**
   - Large percentage display (e.g., "45.2%")
   - Visual: Circular progress indicator or gauge
   - Color coded: green (<70%), yellow (70-85%), red (>85%)
   - Icon: Activity or Cpu icon

2. **Memory Usage Card**
   - Display: "4.0 GB / 8.0 GB"
   - Percentage: "50.0%"
   - Visual: Horizontal progress bar
   - Color coded: green (<80%), yellow (80-90%), red (>90%)
   - Icon: Database or HardDrive icon

3. **Disk Usage Card**
   - Display: "100 GB / 500 GB"
   - Percentage: "20.0%"
   - Visual: Horizontal progress bar
   - Color coded: green (<70%), yellow (70-90%), red (>90%)
   - Icon: HardDrive icon

4. **Network Traffic Card**
   - RX: "1.2 GB" (received)
   - TX: "800 MB" (transmitted)
   - Icons: Download and Upload icons
   - Color: blue theme

**Historical Charts Section:**

Add time range selector above charts:
- Buttons: "1 Hour", "6 Hours", "24 Hours", "7 Days"
- Selected button highlighted

Four line charts showing historical data (last 100 data points):

1. **CPU Usage Over Time**
   - Line chart, smooth curve
   - X-axis: Time (formatted: HH:mm)
   - Y-axis: Percentage (0-100%)
   - Line color: blue
   - Fill gradient: blue fade
   - Show data points on hover with tooltip

2. **Memory Usage Over Time**
   - Line chart
   - Show two lines:
     - Used memory (solid line)
     - Total memory (dashed line)
   - Colors: green and light gray
   - Y-axis: GB
   - Tooltip shows exact values

3. **Disk Usage Over Time**
   - Area chart
   - X-axis: Time
   - Y-axis: GB or %
   - Color: orange/amber
   - Show used vs total

4. **Network Traffic Over Time**
   - Two-line chart
   - RX (download) - blue line
   - TX (upload) - green line
   - Y-axis: MB or GB
   - Legend showing RX/TX

---

## Design Requirements

### Color Scheme
- **Background**: Dark mode preferred (dark gray/slate)
- **Primary**: Blue (#3b82f6)
- **Success/Healthy**: Green (#10b981)
- **Warning/Degraded**: Yellow/Amber (#f59e0b)
- **Error/Critical**: Red (#ef4444)
- **Text**: White/Light gray on dark background
- **Cards**: Darker background with subtle border

### Typography
- **Headings**: Bold, large (text-2xl to text-4xl)
- **Metrics**: Very large for current values (text-4xl to text-6xl)
- **Labels**: Smaller, muted color (text-sm text-gray-400)
- **Body**: Regular (text-base)

### Components Style
- **Cards**: Rounded corners (rounded-lg), subtle shadow
- **Status Badges**: Pill-shaped, colored background with white text
- **Progress Bars**: Smooth, rounded, colored based on value
- **Buttons**: Rounded, solid or outline variants
- **Charts**: Clean grid lines, smooth animations

### Responsiveness
- Mobile: Stack cards vertically, full width
- Tablet: 2 columns for metric cards
- Desktop: 4 columns for metric cards, 2 columns for charts
- Large screens: Keep max-width (e.g., max-w-7xl) for readability

---

## Key Features to Implement

### 1. Auto-Refresh
- Fetch data from API every 10 seconds
- Show loading indicator during fetch
- Display "Last updated: X seconds ago" in the UI
- Pause refresh when user is viewing a chart tooltip (optional)

### 2. Data Formatting
- Format bytes to human-readable (KB, MB, GB, TB)
- Format percentages to 1 decimal place
- Format timestamps to relative time ("2 minutes ago")
- Format chart timestamps to HH:mm format

### 3. Error Handling
- Show error message if API is unreachable
- Retry failed requests
- Display "No data available" if no metrics found
- Fallback values for missing data

### 4. Loading States
- Skeleton loaders for cards while data is loading
- Spinner or progress indicator for initial load
- Smooth transitions when data updates

### 5. Navigation
- Clean navigation bar or sidebar
- Breadcrumbs on detail page
- Back button on detail page
- Smooth page transitions

### 6. Performance
- Debounce API calls
- Cache data for short periods
- Optimize chart rendering
- Lazy load charts on scroll (optional)

---

## Helper Functions Needed

### 1. Format Bytes
```typescript
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}
```

### 2. Format Relative Time
```typescript
function formatRelativeTime(timestamp: string): string {
  const now = new Date();
  const date = new Date(timestamp);
  const seconds = Math.floor((now - date) / 1000);
  
  if (seconds < 60) return `${seconds} seconds ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  return `${days} days ago`;
}
```

### 3. Get Status Color
```typescript
function getStatusColor(status: string): string {
  if (status === 'Healthy') return 'green';
  if (status === 'Degraded') return 'yellow';
  if (status === 'Unreachable') return 'red';
  return 'gray';
}
```

### 4. Get Metric Color (for thresholds)
```typescript
function getMetricColor(metric: string, percentage: number): string {
  if (metric === 'cpu_percent') {
    if (percentage < 70) return 'green';
    if (percentage < 85) return 'yellow';
    return 'red';
  }
  if (metric === 'memory' || metric === 'disk') {
    if (percentage < 80) return 'green';
    if (percentage < 90) return 'yellow';
    return 'red';
  }
  return 'blue';
}
```

---

## Example Data Structure for TypeScript

```typescript
interface Agent {
  id: string;
  name: string;
  last_seen: string;
  status: 'Healthy' | 'Degraded' | 'Unreachable';
}

interface Metric {
  name: string;
  value: number;
  timestamp: string;
}

interface LatestMetrics {
  agent_id: string;
  metrics: Metric[];
}

interface DataPoint {
  timestamp: string;
  value: number;
}

interface HistoricalMetrics {
  agent_id: string;
  metric: string;
  count: number;
  data_points: DataPoint[];
}

interface Stats {
  total_agents: number;
  total_metrics: number;
  total_data_points: number;
}
```

---

## UX Considerations

### Visual Hierarchy
1. Most important info (status, critical metrics) at the top
2. Current values larger and more prominent than historical data
3. Use color to draw attention to issues (red for critical)

### Interactivity
- Hover effects on all clickable elements
- Tooltips showing exact values on chart hover
- Smooth animations for transitions
- Click on server card navigates to detail page

### Accessibility
- Proper semantic HTML
- ARIA labels for icons and interactive elements
- Keyboard navigation support
- Color blind friendly color choices (don't rely only on color)

### Performance Indicators
- Show "Updating..." or pulse animation during refresh
- Display "Last updated" timestamp
- Connection status indicator (connected/disconnected)

---

## Example Component Structure

```
app/
├── page.tsx                    # Overview dashboard
├── server/
│   └── [agentId]/
│       └── page.tsx            # Server detail view
├── components/
│   ├── ServerCard.tsx          # Server overview card
│   ├── MetricCard.tsx          # Current metric display
│   ├── StatusBadge.tsx         # Status indicator
│   ├── MetricChart.tsx         # Reusable chart component
│   └── StatsCards.tsx          # Header statistics
├── lib/
│   ├── api.ts                  # API client functions
│   └── utils.ts                # Helper functions
└── types/
    └── index.ts                # TypeScript types
```

---

## Additional Nice-to-Have Features

1. **Dark/Light Mode Toggle** (prefer dark mode by default)
2. **Search/Filter** servers by name or status
3. **Sort** servers by CPU, memory, or status
4. **Export** data as CSV or JSON
5. **Notifications** when server goes from healthy to degraded
6. **Comparison View** - compare metrics across multiple servers
7. **Alert Banner** at top showing count of degraded/unreachable servers
8. **Time Range Selector** for charts (1h, 6h, 24h, 7d)
9. **Fullscreen Chart Mode** - click to expand chart
10. **Metric Threshold Indicators** - show horizontal lines on charts for warning/critical thresholds

---

## Testing the Dashboard

### Sample Test Data
When testing, you can use these curl commands to populate the server with test data:

```bash
# Send test metrics
curl -X POST http://localhost:8080/api/v1/metrics \
  -H "Content-Type: application/json" \
  -d '{
    "agent_id": "web-server-01",
    "timestamp": "2026-01-27T10:30:45Z",
    "metrics": {
      "cpu_percent": 45.2,
      "memory_used_bytes": 4294967296,
      "memory_total_bytes": 8589934592,
      "disk_used_bytes": 107374182400,
      "disk_total_bytes": 536870912000,
      "network_rx_bytes": 1048576000,
      "network_tx_bytes": 524288000
    }
  }'
```

---

## Final Notes

- **Make it look professional** - this is a DevOps monitoring dashboard that could be used in production
- **Emphasize real-time updates** - the dashboard should feel live and responsive
- **Prioritize clarity** - metrics should be easy to read at a glance
- **Use consistent spacing and sizing** - follow a design system
- **Mobile-first approach** - ensure it works well on all devices
- **Performance matters** - dashboard should update smoothly without lag

The goal is to create a dashboard that a DevOps engineer would be proud to show in an interview or use in production to monitor their infrastructure.

Good luck building this awesome monitoring dashboard! 🚀

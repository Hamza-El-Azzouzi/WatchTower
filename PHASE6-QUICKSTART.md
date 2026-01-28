# 🎯 Phase 6 - Quick Start Guide

## ✅ System is LIVE!

**Backend:** Running on http://localhost:8080  
**Frontend:** Running on http://localhost:3000  
**Alerts Dashboard:** http://localhost:3000/alerts

## 🚨 Current Alert Status

**Active Alerts: 2 Pending**
- Hit Ratio for DB on 01blog-abdeladim (99.81% > 90%)
- Hit Ratio for DB on 01Blog-db (99.90% > 90%)

Both alerts are **pending** and will fire after 5 minutes (300 seconds).

## 📊 Monitoring Status

**Connected Agents: 6**
1. ✅ hkhlifi PC (7 metrics)
2. ✅ 01Blog-db (17 metrics with PostgreSQL)
3. ✅ 01blog-abdeladim (17 metrics with PostgreSQL)
4. ✅ docker-db-01 (7 metrics)
5. ✅ docker-api-01 (7 metrics)
6. ✅ docker-web-01 (7 metrics)

## 🎮 Quick Actions

### View Current Alerts
```bash
# In browser
http://localhost:3000/alerts

# Via API
curl http://localhost:8080/api/v1/alerts | jq
```

### Create Alert Rule
```bash
# In browser - Click "Create Alert Rule" button
http://localhost:3000/alerts/rules/new

# Via API
curl -X POST http://localhost:8080/api/v1/alert-rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "High CPU Alert",
    "description": "Alert when CPU exceeds 80%",
    "metric": "cpu_usage",
    "condition": "GreaterThan",
    "threshold": 80.0,
    "duration_seconds": 30,
    "severity": "Warning",
    "cooldown_seconds": 300
  }'
```

### Test Alert (Quick Fire)
Create a rule with low threshold to see it trigger quickly:

**Option 1: UI (Recommended)**
1. Go to http://localhost:3000/alerts/rules/new
2. Fill in:
   - Name: Test Alert
   - Metric: CPU Usage (%)
   - Condition: > (Greater Than)
   - Threshold: 1
   - Duration: 10
   - Severity: Info
3. Watch alert appear in ~15 seconds!

**Option 2: API**
```bash
curl -X POST http://localhost:8080/api/v1/alert-rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Alert - CPU",
    "metric": "cpu_usage",
    "condition": "GreaterThan",
    "threshold": 1.0,
    "duration_seconds": 10,
    "severity": "Info"
  }'
```

## 📱 UI Features

### Dashboard Tabs
- **Active:** Shows firing + pending alerts (real-time)
- **History:** Shows resolved alerts
- **Rules:** Shows all configured alert rules

### Alert Bell Badge
- Located in left sidebar
- Shows count of active alerts (firing + pending)
- Red badge with pulse animation
- Updates every 10 seconds

### Stats Cards
- **Firing:** Red badge (urgent alerts)
- **Pending:** Yellow badge (conditions being checked)
- **Resolved:** Green badge (past alerts)
- **Rules:** Blue badge (total configured rules)

## 🎨 Alert Severities

| Severity | Color | Emoji | Use Case |
|----------|-------|-------|----------|
| **Info** | Blue | ℹ️ | General notifications |
| **Warning** | Yellow | ⚠️ | Medium priority issues |
| **Critical** | Red | 🔴 | Urgent issues |

## 🔄 Alert Lifecycle

```
1. Rule Condition Met
   ↓
2. PENDING (Duration timer starts)
   ├─ Shows in "Active" tab with yellow badge
   ├─ Timer counts down
   └─ If condition clears → Resolved
   
3. Duration Exceeded
   ↓
4. FIRING (Alert active)
   ├─ Shows in "Active" tab with red badge
   ├─ Alert bell badge increases
   ├─ Notifications sent* (not implemented yet)
   └─ Can be acknowledged
   
5. Condition Clears
   ↓
6. RESOLVED (Alert cleared)
   ├─ Moves to "History" tab
   ├─ Alert bell badge decreases
   └─ After 24h → Automatically deleted
```

## 📊 Available Metrics

**System Metrics:**
- CPU Usage (%)
- Memory Usage (%)
- Disk Usage (%)
- Network RX (bytes/s)
- Network TX (bytes/s)

**Database Metrics:**
- Active DB Connections
- DB Cache Hit Ratio (%)
- DB Slow Queries
- DB Locks Waiting

## 🧪 Testing Scenarios

### Scenario 1: High CPU Alert
```
Metric: CPU Usage (%)
Condition: > 80
Duration: 30 seconds
Result: Fires when CPU >80% for 30+ seconds
```

### Scenario 2: Memory Warning
```
Metric: Memory Usage (%)
Condition: > 85
Duration: 60 seconds
Result: Fires when memory >85% for 1+ minute
```

### Scenario 3: Database Connection Alert
```
Metric: Active DB Connections
Condition: > 100
Duration: 30 seconds
Result: Fires when connections >100 for 30+ seconds
```

### Scenario 4: Low Disk Space
```
Metric: Disk Usage (%)
Condition: > 90
Duration: 120 seconds
Result: Fires when disk >90% for 2+ minutes
```

## 🛠️ Troubleshooting

### Alert Not Firing?
- Check duration hasn't exceeded (pending → firing takes time)
- Verify metric values in Performance dashboard
- Ensure threshold is appropriate
- Check cooldown isn't active

### Alert Bell Not Showing?
- Refresh page (updates every 10 seconds)
- Check browser console for errors
- Verify API is responding: `curl http://localhost:8080/api/v1/alerts`

### Can't Create Rule?
- Check all required fields filled
- Verify server is running: `curl http://localhost:8080/health`
- Check browser console for errors

## 📖 Documentation

- **Full Guide:** [PHASE6-COMPLETE.md](PHASE6-COMPLETE.md)
- **Testing Guide:** [PHASE6-TESTING.md](PHASE6-TESTING.md)

## 🚀 What's Next?

### Decide on Notifications
Once you decide which notification channels to use:
- Discord
- Slack
- Email
- Telegram

I'll implement the notification dispatcher to send alerts automatically!

### Polish Features
- Rule editing UI
- Agent filtering
- Alert sounds
- Export functionality

## 💡 Pro Tips

1. **Start with low thresholds** to test quickly (e.g., CPU > 5%)
2. **Use appropriate durations** to avoid false positives (30-60 seconds)
3. **Set cooldowns** to prevent alert spam (5-15 minutes)
4. **Use severity wisely:**
   - Info: FYI alerts
   - Warning: Action may be needed soon
   - Critical: Immediate action required

## 🎉 Success Checklist

- ✅ Backend server running with alert manager
- ✅ Frontend dashboard accessible
- ✅ 6 agents connected and sending metrics
- ✅ Alert evaluation running every 10 seconds
- ✅ 2 pending alerts already detected
- ✅ Alert bell badge showing in sidebar
- ✅ All 3 dashboard tabs working
- ✅ Alert rule creation form ready

**You're all set! Start testing at http://localhost:3000/alerts** 🎊

---

**Need help?** Check the full documentation or test with a low-threshold rule first!

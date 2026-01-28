# Phase 6 - Alerting System Testing Guide

## ✅ System Status

**Backend Server:** Running on port 8080
- Alert Manager: Initialized ✅
- Alert Evaluation: Running every 10 seconds ✅
- Cleanup Task: Running every hour ✅
- All 6 agents connected ✅

**Frontend Dashboard:** Running on port 3000
- Alerts Dashboard: `/alerts` ✅
- Create Alert Rule: `/alerts/rules/new` ✅
- Alert Bell Badge: Shows active alert count ✅

## 🧪 Testing Steps

### 1. View Alerts Dashboard
```bash
# Navigate to: http://localhost:3000/alerts
```

**Expected:**
- 3 tabs: Active, History, Rules
- Stats cards showing: Firing (0), Pending (0), Resolved (0), Rules (0)
- "All Clear" message (no active alerts)
- "No Rules" message with "Create Alert Rule" button

### 2. Create Your First Alert Rule

**Scenario:** CPU Usage Alert
```
Navigate to: http://localhost:3000/alerts/rules/new

Fill in the form:
- Name: High CPU Usage
- Description: Alert when CPU usage exceeds 80%
- Metric: CPU Usage (%)
- Condition: Greater Than (>)
- Threshold: 80
- Duration: 30 (seconds)
- Severity: Warning ⚠️
- Cooldown: 300 (5 minutes)

Click "Create Alert Rule"
```

**Expected:**
- Redirect to `/alerts`
- Rule appears in "Rules" tab
- Rules count shows "1"

### 3. Test Alert Triggering

**Option A: Natural Trigger (Wait for high CPU)**
- Monitor the dashboard
- Wait for any agent to exceed 80% CPU for 30 seconds
- Alert should appear in "Active" tab as "Pending"
- After 30 seconds, status changes to "Firing"

**Option B: Create a Low Threshold Rule (Faster)**
```
Create another rule:
- Name: Test Alert - Low CPU
- Metric: CPU Usage (%)
- Condition: Greater Than (>)
- Threshold: 5
- Duration: 20
- Severity: Info ℹ️
```

This will trigger immediately since most agents have >5% CPU.

### 4. Verify Alert Lifecycle

**Pending → Firing → Resolved:**

1. **Pending State (Yellow):**
   - Alert appears when condition is met
   - Duration timer starts
   - Shows which agent triggered it

2. **Firing State (Red):**
   - After duration passes, becomes "Firing"
   - Shows "first_triggered_at" and "fired_at" timestamps
   - Badge count increases
   - Alert bell shows number in sidebar

3. **Acknowledge Alert:**
   - Click "Acknowledge" button
   - Alert marked as acknowledged
   - User and timestamp recorded

4. **Resolved State (Green):**
   - When condition no longer met
   - Moves to "History" tab
   - Shows "resolved_at" timestamp

### 5. Advanced Testing

**Test Different Metrics:**
```
Memory Usage:
- Metric: Memory Usage (%)
- Threshold: 85
- Duration: 60

Disk Usage:
- Metric: Disk Usage (%)
- Threshold: 90
- Duration: 120

Database Connections:
- Metric: Active DB Connections
- Threshold: 100
- Duration: 30

Network Traffic:
- Metric: Network RX (bytes/s)
- Threshold: 1000000 (1 MB/s)
```

**Test Different Severities:**
- Info (Blue): General notifications
- Warning (Yellow): Medium priority issues
- Critical (Red): Urgent issues requiring immediate attention

**Test Cooldown:**
1. Create rule with 60 second cooldown
2. Trigger alert
3. Resolve condition briefly
4. Trigger again within 60 seconds
5. Verify alert doesn't re-fire (cooldown active)

### 6. API Testing (Optional)

**Using curl:**

```bash
# List all alert rules
curl http://localhost:8080/api/v1/alert-rules | jq

# Get all alerts
curl http://localhost:8080/api/v1/alerts | jq

# Create alert rule via API
curl -X POST http://localhost:8080/api/v1/alert-rules \
  -H "Content-Type: application/json" \
  -d '{
    "name": "API Test Alert",
    "description": "Created via API",
    "metric": "cpu_usage",
    "condition": "GreaterThan",
    "threshold": 75.0,
    "duration_seconds": 30,
    "severity": "Warning"
  }' | jq

# Delete alert rule
curl -X DELETE http://localhost:8080/api/v1/alert-rules/<rule-id>

# Acknowledge alert
curl -X POST http://localhost:8080/api/v1/alerts/<alert-id>/acknowledge \
  -H "Content-Type: application/json" \
  -d '{
    "acknowledged_by": "admin",
    "note": "Investigating the issue"
  }' | jq
```

## 📊 Expected Metrics

**Connected Agents:**
1. hkhlifi PC (7 metrics)
2. 01Blog-db (17 metrics including PostgreSQL)
3. 01blog-abdeladim (17 metrics including PostgreSQL)
4. Other agents as configured

**Available Metrics for Alerts:**
- CPU Usage (%)
- Memory Usage (%)
- Disk Usage (%)
- Network RX (bytes/s)
- Network TX (bytes/s)
- Active DB Connections
- DB Cache Hit Ratio (%)
- DB Slow Queries
- DB Locks Waiting

## 🔍 Troubleshooting

**No alerts triggering:**
- Check server logs: Evaluation runs every 10 seconds
- Verify agents are sending metrics
- Check threshold values are appropriate
- Ensure duration is not too long

**Alert not resolving:**
- Condition must return to normal state
- Check metric values in Performance dashboard
- Alert manager checks every 10 seconds

**Alert bell not updating:**
- Badge updates every 10 seconds
- Check browser console for API errors
- Verify CORS is enabled on server

**Alert count mismatch:**
- Only "Firing" and "Pending" alerts count in badge
- "Resolved" alerts don't affect badge count

## 🎯 Success Criteria

✅ Can create alert rules via UI
✅ Alerts trigger based on metric conditions
✅ Alert state lifecycle works (Pending → Firing → Resolved)
✅ Alert bell badge shows correct count
✅ Can acknowledge alerts
✅ Alert history preserved
✅ Cooldown prevents alert spam
✅ Different severities display correctly
✅ Alert evaluation runs automatically every 10s

## 📝 Known Limitations

- **Notifications:** Not yet implemented (Discord, Slack, Email, Telegram)
  - User will decide on notification channels
  - Backend structure ready for notification dispatcher
  
- **Rule Editing:** Can only create/delete, not edit existing rules
  - Workaround: Delete and recreate
  
- **Agent Filtering:** Not implemented in rule creation
  - All agents are evaluated for all rules
  - Filter by agent_id can be added later

- **Alert Sounds:** Not implemented
  - Can add browser notification API
  
- **Export:** No CSV export yet
  - Can add export functionality

## 🚀 Next Steps

1. **Test the system** with the scenarios above
2. **Decide on notification channels** (Discord, Slack, Email, Telegram)
3. **Implement notification dispatcher** once channels are decided
4. **Add rule editing UI** for convenience
5. **Add alert detail modal** for full history view
6. **Add alert sounds** for critical alerts
7. **Implement agent filtering** in rules

## 📞 Support

If you encounter any issues during testing:
1. Check server logs in terminal
2. Check browser console for frontend errors
3. Verify all 6 agents are connected
4. Test with low threshold values first (easier to trigger)

---

**Phase 6 Status:** ✅ Core Implementation Complete (90%)
- Backend: Alert evaluation, state management, API ✅
- Frontend: Dashboard, rule creation, alert bell ✅
- Testing: Ready for end-to-end testing ✅
- Notifications: Pending user decision on channels ⏳

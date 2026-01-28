# Phase 6: Alerting System

## Overview
Implement a comprehensive alerting system that monitors metrics and sends notifications when thresholds are breached.

## Features to Implement

### 1. Alert Rules Engine
- Define alert conditions (thresholds, comparisons)
- Support multiple metrics in one rule
- Configurable severity levels (info, warning, critical)
- Time-based conditions (sustained for X minutes)

### 2. Alert Channels
- **Email Notifications** - SMTP integration
- **Webhook** - POST to custom URLs (Slack, Discord, Teams)
- **Dashboard Notifications** - Real-time alerts in UI
- **Log File** - Alert history

### 3. Alert Configuration
```toml
[[alert]]
name = "High CPU Usage"
metric = "cpu_usage"
condition = ">"
threshold = 80.0
duration_seconds = 300  # Sustained for 5 minutes
severity = "warning"
channels = ["email", "webhook"]

[[alert]]
name = "Critical Memory"
metric = "memory_usage"
condition = ">"
threshold = 95.0
duration_seconds = 60
severity = "critical"
channels = ["email", "webhook", "dashboard"]

[[alert]]
name = "Database Connection Pool Saturated"
metric = "db_connections_active"
condition = ">"
threshold_percent = 90  # 90% of max connections
duration_seconds = 120
severity = "critical"
channels = ["webhook"]
```

### 4. Alert States
- **Pending** - Condition met but waiting for duration
- **Firing** - Alert actively triggered
- **Resolved** - Condition no longer met
- **Acknowledged** - User acknowledged the alert
- **Silenced** - Temporarily disabled

### 5. Alert Management Dashboard
- View active alerts
- Alert history with timeline
- Acknowledge/silence alerts
- Configure alert rules via UI
- Test alert channels

## Implementation Plan

### Backend (Rust)
1. **Alert Rule Storage** - In-memory with persistence option
2. **Alert Evaluator** - Background task checking conditions
3. **Alert State Manager** - Track alert lifecycle
4. **Notification Dispatcher** - Send to configured channels
5. **REST API** - CRUD for alert rules and states

### Frontend (Next.js)
1. **Alerts Page** - List active/recent alerts
2. **Alert Rules Config** - Create/edit/delete rules
3. **Alert Bell Icon** - Real-time notification counter
4. **Alert Detail Modal** - Full alert information
5. **Alert History** - Timeline view with filters

### Agent
1. **Optional**: Alert evaluation in agent (edge alerting)
2. **Send alert events** to server

## Database Schema (if adding persistence)
```sql
-- Alert Rules
CREATE TABLE alert_rules (
    id UUID PRIMARY KEY,
    name VARCHAR(255),
    metric VARCHAR(100),
    condition VARCHAR(10),
    threshold FLOAT,
    duration_seconds INT,
    severity VARCHAR(20),
    channels JSON,
    enabled BOOLEAN,
    created_at TIMESTAMP
);

-- Alert History
CREATE TABLE alert_history (
    id UUID PRIMARY KEY,
    rule_id UUID,
    agent_id VARCHAR(100),
    state VARCHAR(20),
    value FLOAT,
    message TEXT,
    triggered_at TIMESTAMP,
    resolved_at TIMESTAMP
);
```

## Notification Examples

### Slack/Discord Webhook
```json
{
  "text": "🚨 Critical Alert",
  "blocks": [
    {
      "type": "header",
      "text": { "type": "plain_text", "text": "High CPU Usage" }
    },
    {
      "type": "section",
      "fields": [
        { "type": "mrkdwn", "text": "*Agent:* docker-web-01" },
        { "type": "mrkdwn", "text": "*Severity:* Critical" },
        { "type": "mrkdwn", "text": "*Value:* 95.3%" },
        { "type": "mrkdwn", "text": "*Threshold:* 80%" }
      ]
    }
  ]
}
```

### Email Template
```
Subject: [CRITICAL] High CPU Usage on docker-web-01

Alert: High CPU Usage
Agent: docker-web-01
Severity: Critical
Triggered: 2026-01-28 13:25:00 UTC

Current Value: 95.3%
Threshold: > 80.0%
Duration: Sustained for 5 minutes

View Details: http://localhost:3000/server/docker-web-01
```

## Alert Routing Logic
```
1. Evaluate metrics every 10 seconds
2. Check if condition is met
3. If met and NEW -> Create pending alert with timestamp
4. If pending and duration passed -> Trigger alert (send notifications)
5. If triggered and condition no longer met -> Resolve alert
6. Prevent duplicate notifications (cooldown period)
```

## Testing Strategy
1. **Unit tests** - Alert evaluation logic
2. **Integration tests** - Notification delivery
3. **Stress tests** - Many alerts simultaneously
4. **Mock webhooks** - Test without real services

## Success Criteria
- [ ] Alert rules can be configured via UI
- [ ] Email notifications work
- [ ] Webhook notifications work
- [ ] Alerts appear in real-time on dashboard
- [ ] Alert history is viewable
- [ ] Alerts can be acknowledged/silenced
- [ ] No duplicate notifications
- [ ] Alert resolution works correctly

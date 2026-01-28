# Enable/Disable Alert Rules Feature

## ✅ Feature Added

You can now **enable or disable alert rules** without deleting them! This is useful for:
- Temporarily silencing alerts during maintenance
- Testing configuration changes
- Keeping rules for future use without triggering

## 🎯 How to Use

### Via UI (Recommended)

1. Go to **http://localhost:3000/alerts**
2. Click the **"Rules"** tab
3. Find the rule you want to enable/disable
4. Click the **"Enable"** or **"Disable"** button
5. Rule status updates immediately!

**Visual Indicators:**
- 🟢 **ENABLED** badge (green) - Rule is active and evaluating
- ⚫ **DISABLED** badge (gray) - Rule is paused and not evaluating

### Via API

**Toggle a rule (enable ↔️ disable):**
```bash
curl -X POST http://localhost:8080/api/v1/alert-rules/{rule-id}/toggle
```

The response returns the updated rule with the new `enabled` status.

## 🔧 What Was Added

### Backend Changes

**1. New Manager Method** (`server/src/alerts/manager.rs`):
```rust
pub async fn toggle_rule(&self, rule_id: &str) -> Option<AlertRule>
```
- Toggles the `enabled` field of a rule
- Returns the updated rule

**2. New API Endpoint** (`server/src/api/mod.rs`):
```rust
POST /api/v1/alert-rules/:rule_id/toggle
```
- Calls the manager's toggle method
- Returns updated AlertRule as JSON

**3. Route Added** (`server/src/main.rs`):
```rust
.route("/api/v1/alert-rules/:rule_id/toggle", post(api::toggle_alert_rule))
```

### Frontend Changes

**1. New API Function** (`lib/alerts-api.ts`):
```typescript
export async function toggleAlertRule(ruleId: string): Promise<AlertRule>
```
- Calls the toggle endpoint
- Returns updated rule

**2. UI Updates** (`app/alerts/page.tsx`):
- Added Power icon (from lucide-react)
- Added "Enable/Disable" button to each rule card
- Button color changes based on state:
  - Gray for Disable (when enabled)
  - Green for Enable (when disabled)
- Loading state shows "Updating..."

## 🎨 Button Appearance

### When Rule is Enabled:
```
┌────────────────┐
│ 🔌 Disable     │ ← Gray button
└────────────────┘
```

### When Rule is Disabled:
```
┌────────────────┐
│ 🔌 Enable      │ ← Green button
└────────────────┘
```

## 📊 Rule Card Layout

```
┌─────────────────────────────────────────────┐
│ Rule Name                    [WARNING] [🟢ENABLED] │
│ Description...                               │
│                                              │
│ Metric: cpu_usage    Condition: > 80       │
│ Duration: 30s        Cooldown: 300s        │
│                                              │
│ [🔌 Disable] [✏️ Edit] [🗑️ Delete]          │
└─────────────────────────────────────────────┘
```

## 🔄 How It Works

### When You Disable a Rule:

1. **Click Disable** button
2. API call: `POST /api/v1/alert-rules/{id}/toggle`
3. Backend sets `enabled = false`
4. Rule badge changes to **DISABLED** (gray)
5. **Alert evaluation skips this rule** (enabled filter already exists)
6. Existing alerts from this rule continue to be tracked
7. No new alerts will be generated

### When You Enable a Rule:

1. **Click Enable** button
2. API call: `POST /api/v1/alert-rules/{id}/toggle`
3. Backend sets `enabled = true`
4. Rule badge changes to **ENABLED** (green)
5. **Alert evaluation includes this rule** on next cycle (10s)
6. New alerts can be generated if conditions are met

## 💡 Use Cases

### 1. Maintenance Window
```
Scenario: Server maintenance scheduled for 30 minutes
Action: Disable all CPU/Memory alerts
Result: No alerts during maintenance
After: Re-enable alerts
```

### 2. Testing Configuration
```
Scenario: Want to adjust thresholds without alerts
Action: 
  1. Disable rule
  2. Edit thresholds
  3. Monitor manually
  4. Re-enable when confident
Result: Clean testing without alert spam
```

### 3. Seasonal Rules
```
Scenario: High traffic only during business hours
Action:
  - Disable "High Traffic" rule after hours
  - Enable during business hours
Result: Relevant alerts only when needed
```

### 4. Rule Library
```
Scenario: Keep rules for different scenarios
Action: 
  - Create rules for various situations
  - Enable only relevant ones
  - Keep others disabled for future use
Result: Rule templates ready to activate
```

## ⚙️ Technical Details

### Alert Evaluation Behavior

The evaluation logic already filters by `enabled`:
```rust
// In evaluate_alerts()
for rule in rules.values() {
    if !rule.enabled {
        continue; // Skip disabled rules
    }
    // ... evaluation logic
}
```

This means:
- ✅ Disabled rules are **not evaluated**
- ✅ No performance impact from disabled rules
- ✅ Existing alerts remain (not deleted)
- ✅ Instant effect (next 10s cycle)

### State Persistence

- Rule state is stored in memory (HashMap)
- Survives until server restart
- After restart, rules return to their defined state
- **Note:** For production, consider persisting to database

## 🧪 Testing

### Test the Toggle Feature:

**1. Find an Existing Rule:**
```bash
curl http://localhost:8080/api/v1/alert-rules | jq '.rules[] | {id, name, enabled}'
```

**2. Toggle It:**
```bash
# Using the rule ID from above
curl -X POST http://localhost:8080/api/v1/alert-rules/{rule-id}/toggle | jq
```

**3. Verify Status Changed:**
```bash
curl http://localhost:8080/api/v1/alert-rules/{rule-id} | jq '.enabled'
```

**4. Toggle Again:**
```bash
curl -X POST http://localhost:8080/api/v1/alert-rules/{rule-id}/toggle | jq
```

### Test in UI:

1. Go to http://localhost:3000/alerts
2. Rules tab
3. Click "Disable" on "Hit Ratio for DB" rule
4. Badge changes to gray "DISABLED"
5. Click "Enable" 
6. Badge changes to green "ENABLED"

## 📝 API Reference

### Toggle Alert Rule

**Endpoint:** `POST /api/v1/alert-rules/:rule_id/toggle`

**Parameters:**
- `rule_id` (path): The ID of the rule to toggle

**Response:**
```json
{
  "id": "680b19ad-c4c4-4534-988e-14a6ad6376f0",
  "name": "Hit Ratio for DB",
  "enabled": false,  // ← Toggled state
  "metric": "db_cache_hit_ratio",
  "condition": "greaterthan",
  "threshold": 90,
  "duration_seconds": 300,
  "severity": "warning",
  "cooldown_seconds": 300,
  "created_at": "2026-01-28T13:17:33.018188578Z"
}
```

**Error Responses:**
- `400 Bad Request` - Rule not found

## 🎯 Summary

✅ **Added:** Enable/Disable button for all alert rules  
✅ **Backend:** New toggle endpoint and manager method  
✅ **Frontend:** Visual toggle button with loading states  
✅ **Behavior:** Disabled rules are not evaluated  
✅ **Persistence:** State maintained until server restart  
✅ **No Breaking Changes:** All existing functionality preserved  

**Try it now:** http://localhost:3000/alerts (Rules tab)

---

**Previous Documentation:**
- [ALERT-MANAGEMENT-GUIDE.md](ALERT-MANAGEMENT-GUIDE.md) - Complete alert management
- [ALERT-FEATURES-SUMMARY.md](ALERT-FEATURES-SUMMARY.md) - All features overview
- [PHASE6-COMPLETE.md](PHASE6-COMPLETE.md) - Phase 6 documentation

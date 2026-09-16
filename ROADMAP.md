# WatchTower Delivery Roadmap

This file tracks implementation state. A checked item must be backed by code and verification;
unchecked items remain part of the active product scope.

## 1. Foundation

- [x] Signed, expiring admin JWTs tied to active admin records
- [x] Explicit bootstrap administrator with no shipped default password
- [x] Hashed API-key storage and forced rotation of legacy plaintext keys
- [x] Agent ID ownership protection during ingestion
- [x] Tenant filtering for agents, metrics, logs, alerts, statistics, and WebSockets
- [x] WebSocket authentication before initial state or events are delivered
- [x] Configurable CORS allow-origin policy
- [x] Secret-free tracked example configuration and required Docker secrets
- [x] Dashboard API/WebSocket URLs work outside localhost builds
- [x] Dashboard lint and TypeScript production build gates
- [x] Correct agent-detail and performance API contracts
- [ ] Rust format, clippy, unit, integration, and container verification
- [ ] Resolve dependency audit findings

Migration note: `20260908000002_hash_api_keys.sql` revokes existing plaintext API keys.
Administrators must generate replacement keys after upgrading.

## 2. Actionable Alerting

- [x] Generic webhook, Slack, Discord, and SMTP email destinations with test delivery
- [x] Persistent delivery history with bounded exponential retry state
- [x] Alert silences and scheduled maintenance windows
- [x] Firing and recovery notifications
- [x] Offline-agent alerts with automatic recovery
- [x] Five-second evaluation cadence with rule duration and cooldown controls
- [x] Correlated incident timeline for alert transitions, acknowledgements, process spikes, and error logs
- [x] Dashboard threshold charts driven by enabled alert rules

## 3. Agent Onboarding

- [ ] One-command installer
- [ ] Server-generated agent configuration
- [ ] Stable generated agent identity
- [ ] Labels and environment metadata
- [x] Agent version, OS, architecture, capabilities, heartbeat and delivery visibility
- [x] Single-use, identity-bound enrollment and two-phase dedicated-key rotation
- [x] Signed, versioned remote collection settings (trusted key installed manually)

## Reliable Agent Delivery

- [x] Disk-backed bounded FIFO for metrics and logs with automatic replay
- [x] Persistent stream/sequence identity and transactional server deduplication
- [x] Reject-newest backpressure, queue diagnostics, retry backoff and replay rate limit
- [x] Restart/crash recovery and exact durable acknowledgement verification
- [x] Database-backed delivery/security tests in the Oracle release gate

## 4. Deeper Monitoring

- [x] Configurable process monitoring (running state, instances, CPU, and memory)
- [x] Read-only systemd service state, PID, uptime, and restart monitoring
- [x] Opt-in read-only Docker CPU, memory, health, uptime, and restart monitoring
- [x] Per-mount capacity, inode, throughput, IOPS, and latency metrics
- [x] Per-interface network rates, packet rates, errors, and drops
- [x] Uptime, load averages, CPU modes, memory pressure, swap, OOM, and TCP metrics

## 5. Operations UX

- [x] Professional alert creation with accessible severity controls and rule preview

- [ ] Historical time-range selection and downsampled queries
- [ ] Agent and time-range comparisons
- [ ] Saved views and filters
- [ ] Cursor-based log pagination
- [ ] Metric and log exports
- [ ] Administrative audit trail
- [ ] Retention controls in the admin UI

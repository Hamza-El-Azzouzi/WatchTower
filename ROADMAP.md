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

- [ ] Webhook destinations and test delivery
- [ ] Persistent delivery history and retry state
- [ ] Alert silences and scheduled maintenance windows
- [ ] Recovery notifications
- [ ] Offline-agent alerts

## 3. Agent Onboarding

- [ ] One-command installer
- [ ] Server-generated agent configuration
- [ ] Stable generated agent identity
- [ ] Labels and environment metadata
- [ ] Agent version, OS, architecture, and upgrade visibility

## 4. Deeper Monitoring

- [ ] Process and service monitoring
- [ ] Container monitoring
- [ ] Per-mount disk metrics
- [ ] Per-interface network rates
- [ ] Uptime and load averages

## 5. Operations UX

- [ ] Historical time-range selection and downsampled queries
- [ ] Agent and time-range comparisons
- [ ] Saved views and filters
- [ ] Cursor-based log pagination
- [ ] Metric and log exports
- [ ] Administrative audit trail
- [ ] Retention controls in the admin UI

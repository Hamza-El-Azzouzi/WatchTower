# WatchTower Implementation Context

Last updated: 2026-09-16

## Architecture

- `agent/`: Rust collector. It sends a flat `HashMap<String, f64>` to
  `POST /api/v1/metrics` and sends file logs separately.
- `server/`: Axum API, PostgreSQL persistence, in-memory recent-series cache,
  alert evaluation, API-key/admin authentication, and authenticated WebSockets.
- `dev-ops-monitoring-dashboard/`: Next.js dashboard. The shared metrics
  WebSocket supplies current agent state and live metric updates.

## Process watch contract

Process watch is configured per agent:

```toml
[process_watch]
enabled = true
names = ["postgres", "nginx"]
```

Names are exact case-insensitive executable-name matches. For privacy and
credential safety, command lines, arguments, and process environments are not
collected. Each configured name produces:

- `process_<safe_name>_running` (`0` or `1`)
- `process_<safe_name>_instances`
- `process_<safe_name>_cpu_usage` (sum across instances)
- `process_<safe_name>_memory_bytes` (sum across instances)

The existing metrics ingestion, storage, tenancy, WebSocket, and alert-rule
paths carry these values without a schema migration. The server-detail page
groups the flat metrics into process rows.

## Security changes in this pass

- Metric payloads now reject empty/oversized batches, unsafe identifiers,
  overlong names, and non-finite values before logging or storage.
- The server caps request bodies at 512 KiB.
- The agent URL-encodes `agent_id` rather than interpolating it into a URL.
- The agent now honors the environment variables advertised by its Docker image,
  including process-watch settings.
- The dashboard emits CSP, clickjacking, MIME-sniffing, referrer, and browser
  permissions headers.
- Vulnerable dashboard packages were upgraded within their current major
  versions; `npm audit` reports zero known vulnerabilities after the update.
- Historical log requests now include the active tenant/admin credential.

## Remaining prioritized work

1. Move browser credentials out of `localStorage` into secure, HttpOnly,
   SameSite cookies via a dashboard backend-for-frontend. This needs an auth
   contract change; CSP reduces but does not remove the current XSS exposure.
2. Extend Linux systemd service telemetry to launchd/Windows equivalents.
3. Exercise notification providers and maintenance policies with production credentials.
4. Add Rust dependency audits and container-level integration tests to CI.
5. Add retention/admin audit controls and cursor-based log pagination.

`ROADMAP.md` remains the source of truth for product completion state.

## UI system

The dashboard uses a restrained operations-console language: deep navy
surfaces, cyan for active telemetry, lime for healthy state, amber for warning,
and rose for critical state. Shared layout behavior lives in `Sidebar`,
`ConditionalLayout`, `PageHeader`, and `app/globals.css`. The shell uses a
fixed 288px desktop rail and a modal navigation drawer below the `lg`
breakpoint. New pages should use the `page-shell`, `surface-panel`, `eyebrow`,
and `metric-track` utilities to preserve spacing and hierarchy.

## Production alerting and host telemetry

Priority 1 now connects configured chart thresholds, offline-agent detection,
firing/recovery delivery (webhook, Discord, Slack, email), exponential retries,
delivery history, silences/maintenance, five-second evaluation, and incident timelines.

Priority 2 adds load/uptime, CPU modes, memory/cache/swap/OOM counters, mount
capacity/inodes and disk I/O, interface traffic/errors/drops, TCP counts,
systemd status/restarts, and sanitized Docker resource/health snapshots.
Structured snapshots are bounded and validated by the server and sent through
the existing authenticated metrics/WebSocket path. Process exploration remains
read-only; command arguments and process environments remain excluded.

On Oracle, an unprivileged agent collects every two seconds. A hardened root
oneshot service writes Docker snapshots every ten seconds to
`/run/watchtower/docker-telemetry.json` (0640 root:watchtower-agent), avoiding
Docker-group privileges for the long-running agent.

Oracle release jobs use native ARM runners for ARM hosts, not QEMU. Docker
dependency cache layers include dummy manifest targets (including the server's
declared performance benchmark), remove them before copying real sources, and
touch the real main source before the final build. This prevents retaining the
placeholder binary. The dashboard lockfile is verified with CI's npm 10.

## Priority 3: reliable delivery (local implementation, 0.2.0)

`agent/src/delivery.rs` supplies a locked, fsynced, bounded disk FIFO for metrics
and logs. The independent uploader replays sequences with capped retry backoff,
jitter and a rate limit; it deletes records only on a matching durable receipt.
PostgreSQL migration `20260916000001_reliable_delivery.sql` adds per-stream
watermarks, heartbeat/upload state, enrollment tokens, rotation state and signed
configuration. Data and watermark updates commit together; old metric replay
does not replace live state. Heartbeats keep replaying agents visible online.

The control plane uses dedicated tenant-bound keys, single-use 15-minute enrollment,
two-phase key rotation, and Ed25519 updates limited to collection interval/flags.
Agents trust an explicitly installed public key, never the endpoint's returned key.
See `deploy/oracle/RELIABLE_DELIVERY.md` for setup and known boundaries (including
enrollment response loss and manual signing-key deployment).

The host page adds `AgentDeliveryPanel`; the Create Alert Rule page uses Lucide
icons, labeled inputs, severity radio cards, validation, channel-state messaging,
and a live policy preview. Remote process controls remain excluded.

## Enterprise alert ownership

Enterprise API keys are tenant credentials, not just read-only dashboard access.
Their owners manage their own rules, alert acknowledgements, notification channels,
delivery history, and silences/maintenance windows. Platform admin credentials manage
keys and cannot mutate enterprise alerting policies. No payment system or additional
enterprise admin panel is introduced.

Migration `20260916000002_tenant_alerting.sql` adds persisted key ownership. All
rule CRUD, chart thresholds, evaluation, notification routing (including offline
alerts), and silences enforce that ownership. A rule without an agent filter means
all agents belonging to its key, never all platform agents. Rule/channel names are unique
within a tenant. Confirmed key rotation transfers policies, channels, silences,
delivery history and the rule cache to the replacement key.

Legacy rules with an exact agent filter inherit that agent's owner. Ambiguous
global rules/channels remain stored but disabled and inaccessible until explicitly
assigned by an operator; this migration does not guess tenant ownership. Customers
can recreate those policies using their enterprise key. Existing channel credentials
are never returned to browsers. Outbound webhook DNS is checked and pinned to public
addresses, with redirects/proxies disabled. SMTP is pinned to a public destination
with certificate validation and mandatory TLS on 465 or STARTTLS on 587.

The disposable-database control-plane integration test exercises two tenants,
cross-tenant access denials, threshold/evaluation isolation, channel references,
offline routing, delivery history, silence isolation, and key-rotation persistence.

## Priority 4: Synthetic monitoring

`server/src/synthetic.rs` runs leased, durable HTTP/HTTPS, TCP, DNS A/AAAA, and
TLS-certificate checks from the server's network vantage point. The dashboard
adds `/synthetic` and an enterprise-only navigation entry, a validated creation
form, pause/resume/delete controls, response-time chart, result/status history,
certificate expiry, and explicit stale/pending/confirming states. Visible pages
refresh every five seconds without overlapping polls; probe intervals are 30–3600s.

Checks belong to API keys and use synthetic agent identities solely for existing
alert/timeline authorization and foreign keys. They consume no host-agent quota,
do not enter the host telemetry store, and cannot receive agent ingestion or
control-plane writes. Dedicated-key rotation transfers their ownership. Each
enterprise may configure 50 active monitors; eight bounded workers share database
leases, use 1–10s total timeouts, and discard results from invalidated leases.

Probe result, incident firing/recovery, timeline event and notification outbox
commit atomically. Consecutive failures (1–10, default 3) open one critical incident;
one success resolves it. Selected tenant channels receive firing and recovery via
the existing retry/history worker. Tenant silences apply. Pausing does not pretend
an outage recovered; deletion retires the incident and cancels pending deliveries.

Results retain timestamps, latency, HTTP status, content-match outcome, public
resolved addresses, expiry date/days and bounded failure reasons, never response
bodies. Storage is capped at 10,000 results per check and 30 days. Deleted monitors
remain tombstoned with history for 30 days before cleanup. HTTP checks are GET-only,
follow no redirects, pin validated public DNS destinations, disable proxies and
limit content validation to 256 KiB. TLS uses certificate-chain/hostname validation;
the default expiry-warning threshold is 14 days. DNS checks use system resolution
for A/AAAA, optionally validating an expected IP; custom resolvers/MX/TXT and
multi-region/browser probes are not implemented.

The disposable control-plane test also exercises synthetic history, stale-lease
deduplication, tenant isolation, consecutive-failure incidents, atomic firing/
recovery deliveries, timelines and tombstoned history. The explicitly enabled
`public_probe_smoke_test` verifies all four real transports, content mismatches,
status mismatches and rejection of the cloud metadata address without an SSRF
test bypass. Oracle deployment now saves a root-only compressed PostgreSQL backup
under `/opt/watchtower/backups` before startup migrations. Backup retention and
operator-reviewed assignment of ambiguous legacy tenant policies remain manual.

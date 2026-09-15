# WatchTower Implementation Context

Last updated: 2026-09-15

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
2. Add service-manager state collection (systemd/launchd/Windows), distinct
   from executable process presence.
3. Add offline-agent alert rules and recovery notifications.
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

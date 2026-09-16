# Synthetic monitoring

Sign in with an enterprise API key and open **Synthetic checks**. Configure a name,
target, interval, timeout, failure count, and optional notification channels.
Configure channels on the Alerts page first. Platform admin tokens cannot create
or modify enterprise monitors.

- HTTP/HTTPS: GET the final URL, validate the exact expected HTTP status (default
  200), optionally match case-sensitive response text. Redirects are not followed.
- TCP: connect to a public hostname/IPv4 address and explicit port.
- DNS: resolve A/AAAA through the server's system resolver; optionally require a
  specific address. This is not authoritative DNS or custom-resolver testing.
- TLS: connect with SNI and normal certificate validation; warn when remaining
  certificate lifetime falls below the configured days (default 14).

Checks execute from Oracle, not from the browser or a global probe network. Private
and metadata destinations are blocked, even when reached through DNS. No private
network allowlist or insecure TLS mode is provided.

An interval is 30–3600 seconds, a timeout 1–10 seconds, and an enterprise may create
50 non-deleted checks. Eight workers use expiring database leases; outage/restart
recovery does not replay missed checks or invent success. The UI labels overdue
results stale. Latency includes DNS/connection and, when configured, bounded body
validation. Sample success rate is not a time-weighted SLA.

Three consecutive failures open an incident by default. A single success resolves
it. Both transitions enqueue notifications atomically with history and use the
existing retries, silences and delivery-history page. Paused monitors retain an
active incident until genuine recovery. Deletion closes it as deleted, cancels
pending notifications, and retains history for 30 days. History is bounded to
10,000 samples per monitor; the UI initially shows the latest 100.

Deployment saves a compressed PostgreSQL dump in `/opt/watchtower/backups` before
migrations. Keep these files private and manage retention off-host. Do not restore
a backup over a live database without an explicit recovery plan.

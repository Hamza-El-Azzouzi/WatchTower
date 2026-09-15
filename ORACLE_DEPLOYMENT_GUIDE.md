# WatchTower deployment on Oracle Cloud

This runbook deploys the WatchTower API, PostgreSQL, and Caddy with Docker
Compose, then installs `monitor-agent` as a native systemd service so it can
observe the Oracle host accurately.

The commands below assume an Ubuntu 22.04 or 24.04 Oracle instance. Run them as
the normal SSH user unless a command explicitly uses `sudo`.

## 1. Before connecting to the instance

Choose two public URLs:

- Dashboard: for example `https://monitor.example.com` (Vercel)
- API: for example `https://api.monitor.example.com` (Oracle)

Create an `A` DNS record for the API hostname pointing to the Oracle instance's
reserved public IPv4 address. If the instance has no configured IPv6 route, do
not create an `AAAA` record.

In the Oracle Cloud VCN Network Security Group or subnet security list, allow:

| Source | Protocol | Port | Purpose |
| --- | --- | --- | --- |
| Your public IP/CIDR | TCP | 22 | SSH administration |
| `0.0.0.0/0` | TCP | 80 | ACME challenge and HTTPS redirect |
| `0.0.0.0/0` | TCP | 443 | API HTTPS and WebSockets |

Do not expose ports 5432 or 8080 in Oracle Cloud or the host firewall. Compose
binds both ports only to `127.0.0.1`; PostgreSQL's loopback binding exists so
the native host agent can collect database statistics.

Before continuing, remove or stop GitLab and verify that nothing owns the web
ports:

```bash
sudo ss -lntp | grep -E ':(80|443|5432|8080)\b' || true
uname -m
cat /etc/os-release
```

`uname -m` will normally report `aarch64` on an Ampere instance or `x86_64` on
an AMD/Intel instance. Building the agent on the instance automatically creates
the correct binary.

## 2. Install host dependencies

Install Docker Engine from Docker's official Ubuntu repository:

```bash
sudo apt update
sudo apt install -y ca-certificates curl git jq openssl build-essential pkg-config libssl-dev rsyslog
sudo systemctl enable --now rsyslog
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc
```

Create `/etc/apt/sources.list.d/docker.sources`:

```bash
sudo nano /etc/apt/sources.list.d/docker.sources
```

Paste this, replacing `UBUNTU_CODENAME` with the value shown by
`/etc/os-release` (`jammy` or `noble`) and `ARCHITECTURE` with the output of
`dpkg --print-architecture` (`amd64` or `arm64`):

```text
Types: deb
URIs: https://download.docker.com/linux/ubuntu
Suites: UBUNTU_CODENAME
Components: stable
Architectures: ARCHITECTURE
Signed-By: /etc/apt/keyrings/docker.asc
```

Install and verify Docker:

```bash
sudo apt update
sudo apt install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo systemctl enable --now docker
sudo docker run --rm hello-world
sudo docker compose version
```

Docker group membership is effectively root access, so the deployment steps use
`sudo docker` rather than adding the SSH user to the `docker` group.

## 3. Download the backend repository

The latest backend and agent changes must first be committed and pushed to the
WatchTower repository. They are separate from the frontend-only repository.

On Oracle:

```bash
sudo install -d -m 0750 -o "$USER" -g "$USER" /opt/watchtower-src
git clone https://github.com/Hamza-El-Azzouzi/WatchTower.git /opt/watchtower-src/repository
cd /opt/watchtower-src/repository
git switch main
git pull --ff-only
```

## 4. Build and configure the server

Build the server image natively for the Oracle architecture:

```bash
cd /opt/watchtower-src/repository
sudo docker build --pull -t watchtower-server:local ./server
```

Install the production Compose and Caddy configuration:

```bash
sudo install -d -m 0750 /opt/watchtower
sudo install -m 0644 deploy/oracle/compose.yml /opt/watchtower/compose.yml
sudo install -m 0644 deploy/oracle/Caddyfile /opt/watchtower/Caddyfile
```

Generate three different random values. Save them in a password manager; do not
commit them:

```bash
openssl rand -hex 32
openssl rand -hex 32
openssl rand -hex 24
```

Create the production environment file:

```bash
sudo install -m 0600 /dev/null /opt/watchtower/.env
sudo nano /opt/watchtower/.env
```

Paste and edit the following. Use the three generated values for the database
password, JWT secret, and administrator password respectively:

```dotenv
SERVER_IMAGE=watchtower-server:local
API_DOMAIN=api.monitor.example.com
DASHBOARD_ORIGIN=https://monitor.example.com
POSTGRES_DB=monitoring
POSTGRES_USER=monitoring_user
POSTGRES_PASSWORD=PASTE_FIRST_RANDOM_VALUE
ADMIN_JWT_SECRET=PASTE_SECOND_RANDOM_VALUE
BOOTSTRAP_ADMIN_USERNAME=admin
BOOTSTRAP_ADMIN_PASSWORD=PASTE_THIRD_RANDOM_VALUE
```

Use only the hostname for `API_DOMAIN`, without `https://` or a trailing slash.
`DASHBOARD_ORIGIN` must be the exact HTTPS origin used by the browser, without a
trailing slash.

Validate and start the stack:

```bash
cd /opt/watchtower
sudo docker compose --env-file .env -f compose.yml config --quiet
sudo docker compose --env-file .env -f compose.yml up -d
sudo docker compose --env-file .env -f compose.yml ps
sudo docker compose --env-file .env -f compose.yml logs --tail=100 server caddy
```

The server automatically runs its SQL migrations during startup. Caddy obtains
and renews the TLS certificate after DNS points to the instance and ports 80 and
443 are reachable.

Verify both paths:

```bash
curl --fail http://127.0.0.1:8080/health
curl --fail https://api.monitor.example.com/health
```

Both responses should report a healthy status. If public HTTPS fails, inspect:

```bash
sudo docker compose --env-file /opt/watchtower/.env -f /opt/watchtower/compose.yml logs --tail=200 caddy
```

## 5. Connect Vercel to the API

In the Vercel project for `monitoring-dashboard`, set these Production
environment variables:

```dotenv
NEXT_PUBLIC_API_URL=https://api.monitor.example.com
NEXT_PUBLIC_WS_URL=wss://api.monitor.example.com
```

Redeploy the frontend after changing them. Ensure `/opt/watchtower/.env` uses the
same frontend origin in `DASHBOARD_ORIGIN`; restart the server if it changes.

## 6. Create the agent API key

Open the Vercel dashboard at `/admin-login` and sign in using
`BOOTSTRAP_ADMIN_USERNAME` and `BOOTSTRAP_ADMIN_PASSWORD`. Open the API Keys
page, create a key named `oracle-production`, set its maximum agents to `1`, and
copy the returned `msk_...` value immediately. Only its hash is stored by the
server and the complete key is shown only when it is created.

If the frontend is not ready, use the API directly:

```bash
curl --fail-with-body -X POST https://api.monitor.example.com/api/v1/admin/login \
  -H 'Content-Type: application/json' \
  --data '{"username":"admin","password":"YOUR_BOOTSTRAP_PASSWORD"}'
```

Copy the `token` from the response, then create the key:

```bash
curl --fail-with-body -X POST https://api.monitor.example.com/api/v1/auth/keys \
  -H 'Content-Type: application/json' \
  -H 'X-Admin-Token: YOUR_ADMIN_TOKEN' \
  --data '{"name":"oracle-production","description":"Oracle production host","expires_in_days":null,"max_agents":1}'
```

Copy the returned `key` value. Do not put either token in shell history on a
shared machine; the examples use placeholders intentionally.

## 7. Build the native agent

Install Rust for the normal SSH user from the official Rust toolchain installer,
then build on Oracle. Building locally avoids CPU-architecture and glibc
compatibility problems:

```bash
cd /tmp
curl --proto '=https' --tlsv1.2 -fsS https://sh.rustup.rs -o rustup-init.sh
sh rustup-init.sh -y --profile minimal
source "$HOME/.cargo/env"
rustup default stable
cd /opt/watchtower-src/repository/agent
cargo build --release --locked
```

Install the binary, locked-down service account, and systemd unit:

```bash
sudo useradd --system --home-dir /nonexistent --shell /usr/sbin/nologin watchtower-agent 2>/dev/null || true
sudo install -d -m 0750 /etc/watchtower
sudo install -m 0755 target/release/monitor-agent /usr/local/bin/monitor-agent
sudo install -m 0644 /opt/watchtower-src/repository/deploy/oracle/watchtower-agent.service /etc/systemd/system/watchtower-agent.service
```

Check that the binary's shared libraries are available:

```bash
ldd /usr/local/bin/monitor-agent | grep 'not found' || true
/usr/local/bin/monitor-agent --version
```

If `ldd` reports a missing library, install that library before starting the
service.

## 8. Configure and start the agent

Create the root-readable environment file:

```bash
sudo install -m 0600 /dev/null /etc/watchtower/agent.env
sudo nano /etc/watchtower/agent.env
```

Paste this and replace the API key:

```dotenv
AGENT_NAME=oracle-production
SERVER_URL=http://127.0.0.1:8080
API_KEY=msk_REPLACE_WITH_CREATED_KEY
COLLECTION_INTERVAL=15
PROCESS_WATCH_ENABLED=true
PROCESS_WATCH_NAMES=monitor-server,postgres,dockerd,caddy
DB_MONITOR_ENABLED=true
DB_MONITOR_TYPE=postgres
DB_MONITOR_HOST=127.0.0.1
DB_MONITOR_PORT=5432
DB_MONITOR_DATABASE=monitoring
DB_MONITOR_USERNAME=monitoring_user
DB_MONITOR_PASSWORD=THE_SAME_POSTGRES_PASSWORD_FROM_/opt/watchtower/.env
LOG_COLLECTION_ENABLED=true
LOG_PATHS=/var/log/syslog,/var/log/auth.log
LOG_BATCH_SIZE=100
LOG_BATCH_INTERVAL_SECONDS=5
RUST_LOG=info
```

The agent sends data over loopback because it runs beside the API. It does not
need to send its API key across the public Internet. The systemd unit grants
only the supplementary `adm` group needed to read Ubuntu logs; it deliberately
does not grant access to the root-equivalent Docker socket. Log collection
starts at the end of each file, so it forwards new entries rather than importing
the entire historical file.

Start and verify it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now watchtower-agent
sudo systemctl status watchtower-agent --no-pager
sudo journalctl -u watchtower-agent -n 100 --no-pager
```

After 15-30 seconds, verify registration:

```bash
curl --fail https://api.monitor.example.com/api/v1/auth/validate \
  -H 'X-API-Key: msk_REPLACE_WITH_CREATED_KEY'

curl --fail https://api.monitor.example.com/api/v1/agents \
  -H 'X-API-Key: msk_REPLACE_WITH_CREATED_KEY'
```

The dashboard should now show `oracle-production`, its live host metrics, and the
configured Process Watch statuses. Generate one harmless entry to verify the log
pipeline, then wait up to one collection interval:

```bash
logger -p authpriv.notice -t watchtower-test "WatchTower log pipeline is working"
sudo journalctl -u watchtower-agent -n 100 --no-pager | grep -E 'Database monitoring|Log collection|Watching log|Failed'
```

Verify that database metrics and durable logs reached PostgreSQL:

```bash
sudo docker exec watchtower-postgres-1 psql -U monitoring_user -d monitoring \
  -c "SELECT metric_name, value, timestamp FROM metrics WHERE agent_id='oracle-production' AND metric_name LIKE 'db_%' ORDER BY timestamp DESC LIMIT 12;"
sudo docker exec watchtower-postgres-1 psql -U monitoring_user -d monitoring \
  -c "SELECT agent_id, level, source, message, timestamp FROM logs ORDER BY timestamp DESC LIMIT 12;"
```

## 9. Operations and recovery

Useful commands:

```bash
sudo docker compose --env-file /opt/watchtower/.env -f /opt/watchtower/compose.yml ps
sudo docker compose --env-file /opt/watchtower/.env -f /opt/watchtower/compose.yml logs -f --tail=100 server
sudo journalctl -u watchtower-agent -f
sudo systemctl restart watchtower-agent
curl --fail https://api.monitor.example.com/health
```

Back up the database regularly:

```bash
sudo install -d -m 0700 /opt/watchtower/backups
sudo docker compose --env-file /opt/watchtower/.env -f /opt/watchtower/compose.yml \
  exec -T postgres pg_dump -U monitoring_user -d monitoring -Fc \
  | sudo tee /opt/watchtower/backups/monitoring.dump >/dev/null
```

The database lives in the Docker volume named similar to
`watchtower_postgres_data`. Never run `docker compose down -v` in production;
the `-v` option deletes the database volume.

## 10. Automatic updates after the first installation

Use the backend repository's GitHub Actions workflow to deploy only successful
pushes to `main`:

1. Run server and agent tests.
2. Build an immutable server image tagged with the Git commit SHA.
3. Push it to GitHub Container Registry.
4. Build the agent for the architecture reported by `uname -m`.
5. Upload the Compose files and agent binary over SSH.
6. Pull and start the new server, wait for `/health`, then atomically replace and
   restart the agent.
7. Restore the previous image and agent binary if health checks fail.

Create a GitHub environment named `production`, restrict it to `main`, and store
these secrets there:

```text
ORACLE_HOST
ORACLE_USER
ORACLE_SSH_PRIVATE_KEY
ORACLE_SSH_KNOWN_HOSTS
POSTGRES_PASSWORD
ADMIN_JWT_SECRET
BOOTSTRAP_ADMIN_PASSWORD
ORACLE_AGENT_API_KEY
```

Store these non-secret environment variables in the same environment:

```text
API_DOMAIN
DASHBOARD_ORIGIN
BOOTSTRAP_ADMIN_USERNAME
ORACLE_AGENT_NAME
PROCESS_WATCH_NAMES
ORACLE_PLATFORM
POSTGRES_DB
POSTGRES_USER
```

Set `ORACLE_PLATFORM` to `linux/arm64` when `uname -m` is `aarch64`; use
`linux/amd64` when it is `x86_64`. `POSTGRES_DB` and `POSTGRES_USER` are optional
variables and default to `monitoring` and `monitoring_user`. Keep SSH password
login and direct root login disabled, and obtain
`ORACLE_SSH_KNOWN_HOSTS` from a trusted connection rather than accepting a new
host key inside the deployment workflow.

The `Deploy server and agent to Oracle` GitHub Actions workflow now performs
this deployment automatically after every push to `main`. It can also be run
manually from the Actions tab. Vercel continues deploying the separate frontend
repository independently.

`ORACLE_PLATFORM` must be configured in the `production` environment. The
workflow validates it before building, and the remote installer checks it again
against `uname -m`. This prevents an AMD64 artifact from being deployed to an
ARM64 instance, or the reverse.

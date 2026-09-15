#!/usr/bin/env bash
set -euo pipefail

release_dir="${1:?release directory is required}"
source "$release_dir/variables.b64"
decode() { printf '%s' "$1" | base64 --decode; }

SERVER_IMAGE="$(decode "$SERVER_IMAGE_B64")"
REGISTRY="$(decode "$REGISTRY_B64")"
REGISTRY_USER="$(decode "$REGISTRY_USER_B64")"
REGISTRY_PASSWORD="$(decode "$REGISTRY_PASSWORD_B64")"
API_DOMAIN="$(decode "$API_DOMAIN_B64")"
DASHBOARD_ORIGIN="$(decode "$DASHBOARD_ORIGIN_B64")"
POSTGRES_DB="$(decode "$POSTGRES_DB_B64")"
POSTGRES_USER="$(decode "$POSTGRES_USER_B64")"
POSTGRES_PASSWORD="$(decode "$POSTGRES_PASSWORD_B64")"
ADMIN_JWT_SECRET="$(decode "$ADMIN_JWT_SECRET_B64")"
BOOTSTRAP_ADMIN_USERNAME="$(decode "$BOOTSTRAP_ADMIN_USERNAME_B64")"
BOOTSTRAP_ADMIN_PASSWORD="$(decode "$BOOTSTRAP_ADMIN_PASSWORD_B64")"
AGENT_API_KEY="$(decode "$AGENT_API_KEY_B64")"
AGENT_NAME="$(decode "$AGENT_NAME_B64")"
PROCESS_WATCH_NAMES="$(decode "$PROCESS_WATCH_NAMES_B64")"
ORACLE_PLATFORM="$(decode "$ORACLE_PLATFORM_B64")"

case "$(uname -m):$ORACLE_PLATFORM" in
  aarch64:linux/arm64|x86_64:linux/amd64) ;;
  *)
    echo "Refusing deployment: host architecture $(uname -m) does not match $ORACLE_PLATFORM." >&2
    exit 1
    ;;
esac

safe_value='^[A-Za-z0-9._~:/@,+-]+$'
for value in "$SERVER_IMAGE" "$API_DOMAIN" "$DASHBOARD_ORIGIN" "$POSTGRES_DB" "$POSTGRES_USER" "$POSTGRES_PASSWORD" "$ADMIN_JWT_SECRET" "$BOOTSTRAP_ADMIN_USERNAME" "$BOOTSTRAP_ADMIN_PASSWORD"; do
  if [[ ! "$value" =~ $safe_value ]]; then
    echo "Deployment values must be URL-safe and may not contain whitespace, quotes, or shell metacharacters." >&2
    exit 1
  fi
done

install_dir=/opt/watchtower
install -d -m 0750 "$install_dir" /etc/watchtower
install -m 0644 "$release_dir/compose.yml" "$install_dir/compose.yml"
install -m 0644 "$release_dir/Caddyfile" "$install_dir/Caddyfile"

if [[ -f "$install_dir/.env" ]]; then
  cp "$install_dir/.env" "$install_dir/.env.previous"
fi
umask 077
{
  printf 'SERVER_IMAGE=%s\n' "$SERVER_IMAGE"
  printf 'API_DOMAIN=%s\n' "$API_DOMAIN"
  printf 'DASHBOARD_ORIGIN=%s\n' "$DASHBOARD_ORIGIN"
  printf 'POSTGRES_DB=%s\n' "$POSTGRES_DB"
  printf 'POSTGRES_USER=%s\n' "$POSTGRES_USER"
  printf 'POSTGRES_PASSWORD=%s\n' "$POSTGRES_PASSWORD"
  printf 'ADMIN_JWT_SECRET=%s\n' "$ADMIN_JWT_SECRET"
  printf 'BOOTSTRAP_ADMIN_USERNAME=%s\n' "$BOOTSTRAP_ADMIN_USERNAME"
  printf 'BOOTSTRAP_ADMIN_PASSWORD=%s\n' "$BOOTSTRAP_ADMIN_PASSWORD"
} > "$install_dir/.env"

docker_config_dir="$(mktemp -d /tmp/watchtower-docker-config.XXXXXX)"
trap 'rm -rf "$docker_config_dir"' EXIT
export DOCKER_CONFIG="$docker_config_dir"
printf '%s' "$REGISTRY_PASSWORD" | docker login "$REGISTRY" --username "$REGISTRY_USER" --password-stdin
cd "$install_dir"
docker compose --env-file .env -f compose.yml pull
docker compose --env-file .env -f compose.yml up -d --remove-orphans

healthy=false
for _ in {1..30}; do
  if curl -fsS http://127.0.0.1:8080/health >/dev/null; then healthy=true; break; fi
  sleep 2
done
if [[ "$healthy" != true ]]; then
  echo "New server failed its health check; rolling back." >&2
  if [[ -f "$install_dir/.env.previous" ]]; then
    mv "$install_dir/.env.previous" "$install_dir/.env"
    docker compose --env-file .env -f compose.yml up -d --remove-orphans
  fi
  exit 1
fi

if [[ -n "$AGENT_API_KEY" ]]; then
  id watchtower-agent >/dev/null 2>&1 || useradd --system --home-dir /nonexistent --shell /usr/sbin/nologin watchtower-agent
  [[ -f /usr/local/bin/monitor-agent ]] && cp /usr/local/bin/monitor-agent /usr/local/bin/monitor-agent.previous || true
  install -m 0755 "$release_dir/monitor-agent" /usr/local/bin/monitor-agent
  install -m 0644 "$release_dir/watchtower-agent.service" /etc/systemd/system/watchtower-agent.service
  {
    printf 'AGENT_NAME=%s\n' "$AGENT_NAME"
    printf 'SERVER_URL=http://127.0.0.1:8080\n'
    printf 'API_KEY=%s\n' "$AGENT_API_KEY"
    printf 'COLLECTION_INTERVAL=2\n'
    printf 'PROCESS_WATCH_ENABLED=true\n'
    printf 'PROCESS_WATCH_NAMES=%s\n' "$PROCESS_WATCH_NAMES"
    printf 'DB_MONITOR_ENABLED=true\n'
    printf 'DB_MONITOR_TYPE=postgres\n'
    printf 'DB_MONITOR_HOST=127.0.0.1\n'
    printf 'DB_MONITOR_PORT=5432\n'
    printf 'DB_MONITOR_INTERVAL_SECONDS=15\n'
    printf 'DB_MONITOR_DATABASE=%s\n' "$POSTGRES_DB"
    printf 'DB_MONITOR_USERNAME=%s\n' "$POSTGRES_USER"
    printf 'DB_MONITOR_PASSWORD=%s\n' "$POSTGRES_PASSWORD"
    printf 'LOG_COLLECTION_ENABLED=true\n'
    printf 'LOG_PATHS=/var/log/syslog,/var/log/auth.log\n'
    printf 'LOG_BATCH_SIZE=100\n'
    printf 'LOG_BATCH_INTERVAL_SECONDS=5\n'
  } > /etc/watchtower/agent.env
  chmod 0600 /etc/watchtower/agent.env
  systemctl daemon-reload
  systemctl enable --now watchtower-agent
  systemctl restart watchtower-agent
  if ! systemctl is-active --quiet watchtower-agent; then
    echo "Agent failed to start; restoring its previous binary." >&2
    if [[ -f /usr/local/bin/monitor-agent.previous ]]; then
      mv /usr/local/bin/monitor-agent.previous /usr/local/bin/monitor-agent
      systemctl restart watchtower-agent
    fi
    exit 1
  fi
else
  echo "ORACLE_AGENT_API_KEY is not configured; server deployed, agent installation skipped."
fi

rm -rf "$release_dir"
docker image prune -f --filter 'until=168h' >/dev/null
echo "WatchTower deployment completed: $SERVER_IMAGE"

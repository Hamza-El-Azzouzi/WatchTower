#!/usr/bin/env bash
set -euo pipefail

required=(ORACLE_HOST ORACLE_USER ORACLE_PLATFORM SSH_PRIVATE_KEY SSH_KNOWN_HOSTS API_DOMAIN DASHBOARD_ORIGIN POSTGRES_PASSWORD ADMIN_JWT_SECRET BOOTSTRAP_ADMIN_PASSWORD)
for name in "${required[@]}"; do
  if [[ -z "${!name:-}" ]]; then
    echo "Required CI/CD variable is missing: $name" >&2
    exit 1
  fi
done

release_dir="$(mktemp -d)"
trap 'rm -rf "$release_dir"' EXIT

cp deploy/oracle/compose.yml "$release_dir/compose.yml"
cp deploy/oracle/Caddyfile "$release_dir/Caddyfile"
cp deploy/oracle/watchtower-agent.service "$release_dir/watchtower-agent.service"
cp deploy/oracle/watchtower-docker-telemetry.service "$release_dir/watchtower-docker-telemetry.service"
cp deploy/oracle/watchtower-docker-telemetry.timer "$release_dir/watchtower-docker-telemetry.timer"
cp deploy/oracle/remote-deploy.sh "$release_dir/remote-deploy.sh"
cp release/monitor-agent "$release_dir/monitor-agent"

encode() { printf '%s' "$1" | base64 | tr -d '\n'; }
umask 077
{
  printf 'SERVER_IMAGE_B64=%s\n' "$(encode "$SERVER_IMAGE")"
  printf 'REGISTRY_B64=%s\n' "$(encode "$CI_REGISTRY")"
  printf 'REGISTRY_USER_B64=%s\n' "$(encode "$CI_REGISTRY_USER")"
  printf 'REGISTRY_PASSWORD_B64=%s\n' "$(encode "$CI_REGISTRY_PASSWORD")"
  printf 'API_DOMAIN_B64=%s\n' "$(encode "$API_DOMAIN")"
  printf 'DASHBOARD_ORIGIN_B64=%s\n' "$(encode "$DASHBOARD_ORIGIN")"
  printf 'POSTGRES_DB_B64=%s\n' "$(encode "${POSTGRES_DB:-monitoring}")"
  printf 'POSTGRES_USER_B64=%s\n' "$(encode "${POSTGRES_USER:-monitoring_user}")"
  printf 'POSTGRES_PASSWORD_B64=%s\n' "$(encode "$POSTGRES_PASSWORD")"
  printf 'ADMIN_JWT_SECRET_B64=%s\n' "$(encode "$ADMIN_JWT_SECRET")"
  printf 'BOOTSTRAP_ADMIN_USERNAME_B64=%s\n' "$(encode "${BOOTSTRAP_ADMIN_USERNAME:-admin}")"
  printf 'BOOTSTRAP_ADMIN_PASSWORD_B64=%s\n' "$(encode "$BOOTSTRAP_ADMIN_PASSWORD")"
  printf 'AGENT_API_KEY_B64=%s\n' "$(encode "${ORACLE_AGENT_API_KEY:-}")"
  printf 'AGENT_NAME_B64=%s\n' "$(encode "${ORACLE_AGENT_NAME:-oracle-production}")"
  printf 'PROCESS_WATCH_NAMES_B64=%s\n' "$(encode "${PROCESS_WATCH_NAMES:-gitlab,postgres,docker,caddy}")"
  printf 'ORACLE_PLATFORM_B64=%s\n' "$(encode "$ORACLE_PLATFORM")"
} > "$release_dir/variables.b64"

target="${ORACLE_USER}@${ORACLE_HOST}"
ssh_options=(-i "$SSH_PRIVATE_KEY" -o IdentitiesOnly=yes -o UserKnownHostsFile="$SSH_KNOWN_HOSTS" -o StrictHostKeyChecking=yes)
remote_dir="/tmp/watchtower-${CI_COMMIT_SHA}"

ssh "${ssh_options[@]}" "$target" "mkdir -p '$remote_dir'"
scp "${ssh_options[@]}" -r "$release_dir/." "$target:$remote_dir/"
ssh "${ssh_options[@]}" "$target" "chmod 700 '$remote_dir/remote-deploy.sh' && sudo '$remote_dir/remote-deploy.sh' '$remote_dir'"

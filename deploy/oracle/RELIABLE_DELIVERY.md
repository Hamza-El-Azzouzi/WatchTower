# Reliable delivery (agent/server 0.2.0)

Deploy the server first: sequenced uploads require the new migration and a durable
acknowledgement. Older servers are deliberately not allowed to acknowledge spool
records. Existing API keys remain valid; enrollment is for new agent identities.

## Delivery contract

- Metrics **and logs** are persisted before upload, then replayed in FIFO order.
- The spool uses atomic rename, file/directory fsync, mode 0700 directories and
  0600 records, and an exclusive process lock. Do not share a spool between hosts.
- An installation retains its random stream ID and increasing sequence across
  restarts. PostgreSQL commits data and the stream watermark in one transaction.
  Duplicate requests return the same durable receipt without inserting data again.
- A durable receipt must match the exact stream and sequence before deletion.
- Historical metric replay does not replace live charts or evaluate stale alerts.
- The default limit is 256 MiB / 100,000 records. At capacity the agent rejects
  new samples rather than evicting unacknowledged data; counters and warnings expose
  this unavoidable loss. Disk exhaustion/write failures also produce explicit errors.
- Replay is limited to 10 uploads/second by default, with exponential backoff and
  jitter on failure. Collection and heartbeat are independent of upload retries.
- Heartbeats report identity, capabilities, queue pressure and configuration revision
  approximately every ten seconds. Last durable upload is recorded by PostgreSQL
  and checkpointed locally. Queue rejection counters currently reset at agent restart.
- Preserve `/var/lib/watchtower-agent` across releases. Containers must mount a
  persistent volume at that path. Never delete pending records to clear an error.
- Server watermarks must be retained with database backups. Restoring an older
  database can require operator reconciliation of already acknowledged samples.

Configuration supports `[delivery]` in TOML: `directory`, `max_bytes`,
`max_records`, and `replay_interval_ms`. Systemd uses `/var/lib/watchtower-agent`.
Environment overrides: `AGENT_SPOOL_DIRECTORY`, `AGENT_SPOOL_MAX_BYTES`,
`AGENT_SPOOL_MAX_RECORDS`. Remote transport requires HTTPS; plain HTTP is supported
only on loopback for the colocated Oracle agent.

## Enroll a new agent

1. An administrator sends `POST /api/v1/agents/enrollment-tokens` with an
   `X-Admin-Token` header and body `{"agent_id":"new-host"}`.
2. The returned token is bound to that identity, stored hashed, expires in 15 minutes,
   and is consumed transactionally with agent/key creation. It is not a metrics key.
3. Configure `AGENT_NAME`, `SERVER_URL`, and `AGENT_SPOOL_DIRECTORY` for the new
   agent. Do not start the service yet. Supply the token without putting it in shell
   history or command-line arguments:

   ```bash
   read -rs -p 'Enrollment token: ' AGENT_ENROLLMENT_TOKEN
   export AGENT_ENROLLMENT_TOKEN
   monitor-agent enroll
   unset AGENT_ENROLLMENT_TOKEN
   ```

   Run as the same service user that owns the spool. The command stores the new key
   in `credentials.json` (0600), never prints it. Start the agent normally afterward.
4. Stored credentials take precedence over the legacy `API_KEY` environment value.
   Keep the agent identity and spool directory consistent across service/CLI runs.

If the enrollment response is lost **after commit**, the token remains consumed:
an administrator must reconcile the newly created identity/key before reenrollment.
Enrollment is intentionally not an unrestricted automatic identity-reset mechanism.

## Rotate an existing dedicated agent key

Stop the agent service, then run `monitor-agent rotate-key` as its service user,
using the same configuration/environment and spool directory; restart the service.
Keys shared by multiple agents cannot be rotated through this endpoint.

The CLI requests a pending 15-minute key, writes old + pending credentials atomically,
confirms using the new key, and only then replaces the local credential. The server
atomically transfers ownership and revokes the old key on confirmation. Confirmation
is idempotent. A pending local confirmation is retried after restart, without stopping
offline collection. If it expires unconfirmed, rerun rotation while the old key is
still valid. Do not rotate the GitHub secret alone: it would not transfer ownership.

Endpoints: `POST /api/v1/agents/{id}/rotate-key` (current agent key),
`POST /api/v1/agents/{id}/confirm-key` (pending new key).

## Enable signed remote configuration

This is disabled on agents until a trusted public key is explicitly installed.
Never trust a public key returned by the configuration endpoint itself.

1. Generate a pair using the new server utility. Private key: Ed25519 PKCS#8 DER;
   public key: raw 32-byte Ed25519. Neither output file may already exist:

   ```bash
   monitor-server generate-config-key \
     --private-output config-signing.pk8 \
     --public-output config-signing.pub
   ```

2. On Oracle, install the private key at `/opt/watchtower/keys/config-signing.pk8`.
   The server container mounts this directory read-only at `/run/watchtower-keys`.
   The private file and parent directory must be readable only by the server's UID
   1000 (0600/0700). Do not add the private key to Git, the frontend, or the agent.
3. Install only the public key at `/etc/watchtower/config-signing.pub`, root-owned,
   readable by `watchtower-agent` (0640). Configure
   `AGENT_CONFIG_PUBLIC_KEY_FILE=/etc/watchtower/config-signing.pub` in a systemd
   drop-in and restart. Future Oracle deployments detect this installed file.
4. Administrator: `PUT /api/v1/agents/{id}/config` with `X-Admin-Token` and body:

   ```json
   {"interval_seconds":2,"collect_cpu":true,"collect_memory":true,
    "collect_disk":true,"collect_network":true}
   ```

5. The server signs an identity-bound, monotonically versioned update with a seven-day
   acceptance deadline. Agents poll every ten seconds, verify against the installed
   key, reject expired/wrong-target/rollback/tampered updates, checkpoint acceptance,
   and apply collection settings without restart. An already accepted configuration
   survives restart even after its acceptance deadline. Unknown settings are rejected.

No remote commands, process killing, file paths, credentials or server-URL changes
are accepted. Signing-key replacement currently requires trusted manual deployment;
there is no automatic trust-key rotation protocol yet.

## Visibility and verification

`GET /api/v1/agents/{id}/delivery` is tenant-scoped (or administrator-only access).
The host detail page displays this status. Health metadata is self-reported except
for server-recorded heartbeat and upload timestamps; it is not host attestation.

The Oracle deployment verification job runs database-backed tests for concurrent
deduplication, transaction rollback/retry, log replay, one-time/expired enrollment,
tenant isolation, key rotation and signed configuration revisions. Local command,
against a **disposable** database named `watchtower_test`:

```bash
cargo test --locked --manifest-path server/Cargo.toml \
  reliable_delivery_control_plane -- --ignored
```

Set `WATCHTOWER_TEST_DATABASE_URL` and a test `ADMIN_JWT_SECRET` beforehand.
The fixture creates/drops a failure-injection trigger; never point it at production.

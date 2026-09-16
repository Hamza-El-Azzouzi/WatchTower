use crate::{
    config::Config,
    delivery::{atomic_write, DiskQueue},
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Serialize, Deserialize)]
struct Credentials {
    api_key: String,
    #[serde(default)]
    pending_key: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteSettings {
    pub interval_seconds: u64,
    pub collect_cpu: bool,
    pub collect_memory: bool,
    pub collect_disk: bool,
    pub collect_network: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RemotePayload {
    pub agent_id: String,
    pub revision: u64,
    pub expires_at: DateTime<Utc>,
    pub settings: RemoteSettings,
}
#[derive(Deserialize, Serialize)]
pub struct Envelope {
    pub payload: String,
    pub signature: String,
}

pub fn verify(envelope: &Envelope, key: &[u8], id: &str, previous: u64) -> Result<RemotePayload> {
    UnparsedPublicKey::new(&ED25519, key)
        .verify(
            envelope.payload.as_bytes(),
            &STANDARD.decode(&envelope.signature)?,
        )
        .map_err(|_| anyhow::anyhow!("configuration signature is invalid"))?;
    let payload: RemotePayload = serde_json::from_str(&envelope.payload)?;
    if payload.agent_id != id
        || payload.revision <= previous
        || payload.expires_at <= Utc::now()
        || !(1..=300).contains(&payload.settings.interval_seconds)
    {
        bail!("configuration target, revision, expiry or interval rejected");
    }
    Ok(payload)
}
async fn post(
    config: &Config,
    path: &str,
    key: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value> {
    validate_url(&config.server.url)?;
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()?
        .post(format!(
            "{}{}",
            config.server.url.trim_end_matches('/'),
            path
        ))
        .bearer_auth(key)
        .json(&payload)
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("agent control request rejected: {}", response.status());
    }
    bounded_json(response).await
}

pub async fn bounded_json(mut response: reqwest::Response) -> Result<serde_json::Value> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > 64 * 1024 {
            bail!("control response exceeds 64 KiB limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn validate_url(url: &str) -> Result<()> {
    let url = reqwest::Url::parse(url)?;
    if !url.username().is_empty() || url.password().is_some() {
        bail!("server URL must not contain credentials");
    }
    let local = matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "[::1]" | "::1")
    );
    if url.scheme() != "https" && !(url.scheme() == "http" && local) {
        bail!("remote agent delivery requires HTTPS (HTTP is permitted only on loopback)");
    }
    Ok(())
}
pub async fn enroll(config: &Config) -> Result<()> {
    let _queue = DiskQueue::open(
        config.delivery.directory.clone(),
        config.delivery.max_bytes,
        config.delivery.max_records,
    )?;
    let path = config.delivery.directory.join("credentials.json");
    if path.exists() {
        bail!("credentials already exist; use rotate-key instead");
    }
    let token = std::env::var("AGENT_ENROLLMENT_TOKEN")
        .context("set AGENT_ENROLLMENT_TOKEN to a short-lived enrollment token")?;
    let response = post(
        config,
        "/api/v1/agents/enroll",
        &token,
        serde_json::json!({"agent_id":config.agent.name}),
    )
    .await?;
    let credentials = Credentials {
        api_key: response["api_key"]
            .as_str()
            .context("missing enrollment key")?
            .into(),
        pending_key: None,
    };
    atomic_write(&path, &serde_json::to_vec(&credentials)?)?;
    tracing::info!("Enrollment complete; credential stored in restricted agent state directory");
    Ok(())
}
fn read_credentials(path: &Path) -> Result<Credentials> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
        bail!("agent credentials must be a regular file with mode 0600");
    }
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
pub async fn load_credentials(config: &mut Config) -> Result<()> {
    let path = config.delivery.directory.join("credentials.json");
    if !path.exists() {
        return Ok(());
    }
    let mut credentials = read_credentials(&path)?;
    if let Some(pending) = credentials.pending_key.clone() {
        let confirmation = post(
            config,
            &format!("/api/v1/agents/{}/confirm-key", config.agent.name),
            &pending,
            serde_json::json!({}),
        )
        .await;
        if let Err(error) = confirmation {
            tracing::warn!(
                "Key confirmation pending; collecting offline while control plane retries: {error}"
            );
            config.server.api_key = Some(pending);
            return Ok(());
        }
        credentials.api_key = pending;
        credentials.pending_key = None;
        atomic_write(&path, &serde_json::to_vec(&credentials)?)?;
    }
    config.server.api_key = Some(credentials.api_key);
    Ok(())
}
pub async fn rotate(config: &Config) -> Result<()> {
    let _queue = DiskQueue::open(
        config.delivery.directory.clone(),
        config.delivery.max_bytes,
        config.delivery.max_records,
    )?;
    let mut resolved = config.clone();
    load_credentials(&mut resolved).await?;
    let config = &resolved;
    let path = config.delivery.directory.join("credentials.json");
    let persisted = path.exists().then(|| read_credentials(&path)).transpose()?;
    let key = persisted
        .as_ref()
        .map(|credentials| &credentials.api_key)
        .or(config.server.api_key.as_ref())
        .context("an existing API key is required")?;
    let response = post(
        config,
        &format!("/api/v1/agents/{}/rotate-key", config.agent.name),
        key,
        serde_json::json!({}),
    )
    .await?;
    let pending = response["api_key"]
        .as_str()
        .context("missing rotation key")?
        .to_string();
    // Persist both keys before confirming: recovery after a crash is idempotent.
    atomic_write(
        &path,
        &serde_json::to_vec(&Credentials {
            api_key: key.clone(),
            pending_key: Some(pending.clone()),
        })?,
    )?;
    post(
        config,
        &format!("/api/v1/agents/{}/confirm-key", config.agent.name),
        &pending,
        serde_json::json!({}),
    )
    .await?;
    atomic_write(
        &path,
        &serde_json::to_vec(&Credentials {
            api_key: pending,
            pending_key: None,
        })?,
    )?;
    tracing::info!("Agent key rotated; old key revoked after durable local persistence");
    Ok(())
}

pub fn start(
    config: Config,
    queue: Arc<Mutex<DiskQueue>>,
    settings: Arc<Mutex<Option<RemotePayload>>>,
) -> Result<()> {
    let trusted_key = std::env::var("AGENT_CONFIG_PUBLIC_KEY_FILE")
        .ok()
        .map(|path| -> Result<Vec<u8>> {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::symlink_metadata(&path)?;
            if !metadata.is_file() || metadata.permissions().mode() & 0o022 != 0 {
                bail!("trusted configuration key must be a regular file, not group/world writable");
            }
            let key = std::fs::read(path)?;
            if key.len() != 32 {
                bail!("trusted configuration public key must contain 32 raw bytes");
            }
            Ok(key)
        })
        .transpose()?;
    let checkpoint = config.delivery.directory.join("remote-config.json");
    let mut revision = 0;
    if let (Some(key), true) = (trusted_key.as_ref(), checkpoint.exists()) {
        let envelope: Envelope = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        // Preserve the highest accepted revision even when a saved update expires.
        UnparsedPublicKey::new(&ED25519, key)
            .verify(
                envelope.payload.as_bytes(),
                &STANDARD.decode(&envelope.signature)?,
            )
            .map_err(|_| anyhow::anyhow!("saved configuration signature is invalid"))?;
        let payload: RemotePayload = serde_json::from_str(&envelope.payload)?;
        if payload.agent_id != config.agent.name {
            bail!("saved configuration targets another agent");
        }
        revision = payload.revision;
        if !(1..=300).contains(&payload.settings.interval_seconds) {
            bail!("saved configuration interval is invalid");
        }
        // Expiry is an acceptance deadline, not a lease on an already accepted setting.
        *settings.lock().expect("config lock poisoned") = Some(payload);
    }
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                tracing::error!("Control client initialization failed: {error}");
                return;
            }
        };
        loop {
            let credentials_path = config.delivery.directory.join("credentials.json");
            if credentials_path.exists() {
                let result: Result<()> = async {
                    let mut credentials = read_credentials(&credentials_path)?;
                    if let Some(pending) = credentials.pending_key.clone() {
                        post(
                            &config,
                            &format!("/api/v1/agents/{}/confirm-key", config.agent.name),
                            &pending,
                            serde_json::json!({}),
                        )
                        .await?;
                        credentials.api_key = pending;
                        credentials.pending_key = None;
                        atomic_write(&credentials_path, &serde_json::to_vec(&credentials)?)?;
                    }
                    Ok(())
                }
                .await;
                if let Err(error) = result {
                    tracing::warn!("Pending credential confirmation will retry: {error}");
                }
            }
            let status = queue.lock().expect("spool lock poisoned").status();
            if let Ok((records, bytes, last_success)) = status {
                let dropped = queue.lock().expect("spool lock poisoned").dropped_samples;
                let capabilities = vec![
                    "disk_queue",
                    "sequence_deduplication",
                    "host_metrics",
                    "process_explorer_read_only",
                    "systemd",
                    "docker_snapshot",
                    "signed_configuration",
                ];
                let os = sysinfo::System::long_os_version()
                    .unwrap_or_else(|| std::env::consts::OS.into());
                let report = serde_json::json!({"agent_id":config.agent.name,"version":env!("CARGO_PKG_VERSION"),"architecture":std::env::consts::ARCH,"os":os,"capabilities":capabilities,"queue_records":records,"queue_bytes":bytes,"dropped_samples":dropped,"last_successful_upload":last_success,"config_revision":revision});
                if let Some(key) = config.server.api_key.as_ref() {
                    if let Err(error) = post(&config, "/api/v1/agents/heartbeat", key, report).await
                    {
                        tracing::warn!("Heartbeat failed: {error}");
                    }
                    if let Some(trusted_key) = trusted_key.as_ref() {
                        let result: Result<()> = async {
                            let response = client
                                .get(format!(
                                    "{}/api/v1/agents/{}/config",
                                    config.server.url.trim_end_matches('/'),
                                    config.agent.name
                                ))
                                .bearer_auth(key)
                                .send()
                                .await?
                                .error_for_status()?;
                            let value = bounded_json(response).await?;
                            if value.is_null() {
                                return Ok(());
                            }
                            let envelope: Envelope = serde_json::from_value(value)?;
                            let candidate: RemotePayload = serde_json::from_str(&envelope.payload)?;
                            if candidate.revision == revision {
                                return Ok(());
                            }
                            let payload =
                                verify(&envelope, trusted_key, &config.agent.name, revision)?;
                            atomic_write(&checkpoint, &serde_json::to_vec(&envelope)?)?;
                            revision = payload.revision;
                            *settings.lock().expect("config lock poisoned") = Some(payload);
                            Ok(())
                        }
                        .await;
                        if let Err(error) = result {
                            tracing::warn!("Remote configuration rejected: {error}");
                        }
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    });
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn verifies_signature_target_expiry_and_rollback() {
        use ring::signature::{Ed25519KeyPair, KeyPair};
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let payload=serde_json::json!({"agent_id":"test","revision":2,"expires_at":Utc::now()+chrono::Duration::hours(1),"settings":{"interval_seconds":2,"collect_cpu":true,"collect_memory":true,"collect_disk":true,"collect_network":true}}).to_string();
        let envelope = Envelope {
            signature: STANDARD.encode(key.sign(payload.as_bytes()).as_ref()),
            payload,
        };
        assert!(verify(&envelope, key.public_key().as_ref(), "test", 1).is_ok());
        assert!(verify(&envelope, key.public_key().as_ref(), "other", 1).is_err());
        assert!(verify(&envelope, key.public_key().as_ref(), "test", 2).is_err());
        let mut expired: serde_json::Value = serde_json::from_str(&envelope.payload).unwrap();
        expired["expires_at"] =
            serde_json::to_value(Utc::now() - chrono::Duration::seconds(1)).unwrap();
        let expired = expired.to_string();
        let expired = Envelope {
            signature: STANDARD.encode(key.sign(expired.as_bytes()).as_ref()),
            payload: expired,
        };
        assert!(verify(&expired, key.public_key().as_ref(), "test", 1).is_err());
        let altered = Envelope {
            payload: envelope.payload.replace("test", "evil"),
            signature: envelope.signature,
        };
        assert!(verify(&altered, key.public_key().as_ref(), "evil", 1).is_err());
    }
    #[test]
    fn rejects_insecure_remote_transport() {
        assert!(validate_url("http://127.0.0.1:8080").is_ok());
        assert!(validate_url("https://api.example.com").is_ok());
        assert!(validate_url("http://api.example.com").is_err());
        assert!(validate_url("https://secret@example.com").is_err());
    }
}

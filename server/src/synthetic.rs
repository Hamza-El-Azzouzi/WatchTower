//! Bounded, leased synthetic probes from the server's network vantage point.
use crate::alerts::notifications::{alert_payload, public_addresses, validate_webhook_url};
use crate::{
    alerts::{Alert, AlertCondition, AlertEventType, AlertSeverity, AlertState, AlertTransition},
    api::AppState,
};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::net::TcpStream;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Http,
    Tcp,
    Dns,
    Tls,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpec {
    pub name: String,
    pub kind: CheckKind,
    pub target: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default = "interval_default")]
    pub interval_seconds: u64,
    #[serde(default = "timeout_default")]
    pub timeout_seconds: u64,
    #[serde(default = "failures_default")]
    pub failure_threshold: u32,
    #[serde(default = "status_default")]
    pub expected_status: u16,
    #[serde(default)]
    pub expected_content: Option<String>,
    #[serde(default = "dns_default")]
    pub dns_record_type: String,
    #[serde(default)]
    pub expected_dns_value: Option<String>,
    #[serde(default = "expiry_default")]
    pub tls_expiry_days: u32,
    #[serde(default)]
    pub channels: Vec<String>,
}
fn interval_default() -> u64 {
    60
}
fn timeout_default() -> u64 {
    5
}
fn failures_default() -> u32 {
    3
}
fn status_default() -> u16 {
    200
}
fn dns_default() -> String {
    "A".into()
}
fn expiry_default() -> u32 {
    14
}

impl CheckSpec {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty()
            || self.name.len() > 128
            || self.target.len() > 2048
            || !(30..=3600).contains(&self.interval_seconds)
            || !(1..=10).contains(&self.timeout_seconds)
            || !(1..=10).contains(&self.failure_threshold)
            || !(100..=599).contains(&self.expected_status)
            || self
                .expected_content
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > 4096)
            || self.tls_expiry_days > 365
            || self.channels.len() > 64
        {
            return Err(anyhow!("Invalid check: name ≤128 characters, interval 30–3600s, timeout 1–10s, failures 1–10, content ≤4096 bytes"));
        }
        if self.kind == CheckKind::Http {
            let url = reqwest::Url::parse(&self.target)?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
            {
                return Err(anyhow!("HTTP target must be an HTTP/HTTPS URL without embedded credentials or fragments"));
            }
            // Apply the same local-address checks to HTTP as HTTPS.
            let mut secure = url.clone();
            secure
                .set_scheme("https")
                .map_err(|_| anyhow!("Invalid URL"))?;
            validate_webhook_url(secure.as_str())?;
        } else {
            if self.target.is_empty()
                || self.target.len() > 253
                || self.target.contains(['/', '@', ':'])
                || !self
                    .target
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
            {
                return Err(anyhow!(
                    "Target must be a hostname or IPv4 address, without a URL or port"
                ));
            }
            validate_webhook_url(&format!("https://{}/", self.target))?;
            if self.kind == CheckKind::Tcp && self.port.is_none() || self.port == Some(0) {
                return Err(anyhow!("TCP checks require a nonzero port"));
            }
        }
        if !matches!(self.dns_record_type.as_str(), "A" | "AAAA") {
            return Err(anyhow!("DNS record type must be A or AAAA"));
        }
        if let Some(expected) = &self.expected_dns_value {
            let ip: IpAddr = expected
                .parse()
                .context("Expected DNS value must be an IP address")?;
            if (self.dns_record_type == "A") != ip.is_ipv4() {
                return Err(anyhow!("Expected DNS value does not match record type"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub checked_at: DateTime<Utc>,
    pub success: bool,
    pub response_time_ms: f64,
    pub status_code: Option<u16>,
    pub content_matched: Option<bool>,
    pub resolved_addresses: Vec<String>,
    pub tls_expires_at: Option<DateTime<Utc>>,
    pub tls_days_remaining: Option<f64>,
    pub error: Option<String>,
}

pub async fn probe(spec: &CheckSpec) -> ProbeResult {
    let start = Instant::now();
    let mut result = ProbeResult {
        checked_at: Utc::now(),
        success: false,
        response_time_ms: 0.0,
        status_code: None,
        content_matched: None,
        resolved_addresses: vec![],
        tls_expires_at: None,
        tls_days_remaining: None,
        error: None,
    };
    let outcome = tokio::time::timeout(
        Duration::from_secs(spec.timeout_seconds),
        execute(spec, &mut result),
    )
    .await;
    result.error = match outcome {
        Ok(Ok(())) => {
            result.success = true;
            None
        }
        Ok(Err(error)) => Some(error.to_string().chars().take(512).collect()),
        Err(_) => Some("Probe timed out".into()),
    };
    result.response_time_ms = start.elapsed().as_secs_f64() * 1000.0;
    result
}

async fn execute(spec: &CheckSpec, result: &mut ProbeResult) -> Result<()> {
    let (host, port) = if spec.kind == CheckKind::Http {
        let url = reqwest::Url::parse(&spec.target)?;
        (
            url.host_str()
                .context("Missing HTTP host")?
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned(),
            url.port_or_known_default().context("Missing HTTP port")?,
        )
    } else {
        (spec.target.clone(), spec.port.unwrap_or(443))
    };
    let addresses = public_addresses(&host, port).await?;
    result.resolved_addresses = addresses.iter().map(|addr| addr.ip().to_string()).collect();
    result.resolved_addresses.sort();
    result.resolved_addresses.dedup();
    match spec.kind {
        CheckKind::Dns => {
            let matching: Vec<_> = addresses
                .iter()
                .map(|addr| addr.ip())
                .filter(|ip| (spec.dns_record_type == "A") == ip.is_ipv4())
                .collect();
            if matching.is_empty() {
                return Err(anyhow!("DNS returned no {} records", spec.dns_record_type));
            }
            if let Some(value) = &spec.expected_dns_value {
                if !matching.contains(&value.parse::<IpAddr>()?) {
                    return Err(anyhow!("DNS response does not contain expected address"));
                }
            }
        }
        CheckKind::Tcp => {
            TcpStream::connect(addresses.as_slice()).await?;
        }
        CheckKind::Tls => {
            let tcp = TcpStream::connect(addresses.as_slice()).await?;
            let connector = tokio_native_tls::TlsConnector::from(native_tls::TlsConnector::new()?);
            let stream = connector.connect(&host, tcp).await?;
            let cert = stream
                .get_ref()
                .peer_certificate()?
                .context("Peer did not return a certificate")?;
            let x509 = openssl::x509::X509::from_der(&cert.to_der()?)?;
            let now = openssl::asn1::Asn1Time::from_unix(Utc::now().timestamp())?;
            let diff = now.diff(x509.not_after())?;
            let seconds = i64::from(diff.days) * 86400 + i64::from(diff.secs);
            let days = seconds as f64 / 86400.0;
            result.tls_days_remaining = Some(days);
            result.tls_expires_at = Some(Utc::now() + chrono::Duration::seconds(seconds));
            if days <= f64::from(spec.tls_expiry_days) {
                return Err(anyhow!(
                    "TLS certificate expires in {days:.1} days (threshold {} days)",
                    spec.tls_expiry_days
                ));
            }
        }
        CheckKind::Http => {
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(spec.timeout_seconds))
                .resolve_to_addrs(&host, &addresses)
                .user_agent("WatchTower-Synthetic/0.2")
                .build()?;
            let mut response = client.get(&spec.target).send().await?;
            let status = response.status().as_u16();
            result.status_code = Some(status);
            if let Some(expected) = &spec.expected_content {
                let mut body = Vec::new();
                while let Some(chunk) = response.chunk().await? {
                    if body.len() + chunk.len() > 256 * 1024 {
                        return Err(anyhow!("Response content exceeds 256 KiB validation limit"));
                    }
                    body.extend_from_slice(&chunk);
                }
                let matched = body
                    .windows(expected.len())
                    .any(|window| window == expected.as_bytes());
                result.content_matched = Some(matched);
                if !matched {
                    return Err(anyhow!("Expected response content was not found"));
                }
            }
            if status != spec.expected_status {
                return Err(anyhow!(
                    "HTTP returned {status}; expected {}",
                    spec.expected_status
                ));
            }
        }
    }
    Ok(())
}

pub async fn run(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let slots = Arc::new(tokio::sync::Semaphore::new(8));
    let mut cleanup_at = Instant::now() - Duration::from_secs(300);
    loop {
        interval.tick().await;
        if cleanup_at.elapsed() >= Duration::from_secs(300) {
            if let Some(db) = &state.database {
                if let Err(error) = sqlx::query("DELETE FROM agents WHERE id IN (SELECT agent_id FROM synthetic_checks WHERE deleted_at<now()-interval '30 days')")
                    .execute(db.pool()).await { tracing::error!(%error,"Synthetic tombstone cleanup failed"); }
                if let Err(error) = sqlx::query("DELETE FROM synthetic_results WHERE id IN (SELECT id FROM synthetic_results WHERE checked_at<now()-interval '30 days' LIMIT 10000)")
                    .execute(db.pool()).await { tracing::error!(%error,"Synthetic history cleanup failed"); }
            }
            cleanup_at = Instant::now();
        }
        for _ in 0..8 {
            let Ok(permit) = slots.clone().try_acquire_owned() else {
                break;
            };
            let Some(db) = state.database.as_ref() else {
                return;
            };
            let lease = uuid::Uuid::new_v4().to_string();
            let claimed=sqlx::query("UPDATE synthetic_checks SET lease_token=$1, leased_until=now()+interval '30 seconds' WHERE id=(SELECT c.id FROM synthetic_checks c JOIN api_keys k ON k.id=c.owner_api_key_id WHERE c.enabled AND NOT k.revoked AND (k.expires_at IS NULL OR k.expires_at>now()) AND c.next_run_at<=now() AND (c.leased_until IS NULL OR c.leased_until<now()) ORDER BY c.next_run_at FOR UPDATE OF c SKIP LOCKED LIMIT 1) RETURNING id,spec")
                .bind(&lease).fetch_optional(db.pool()).await;
            match claimed {
                Ok(Some(row)) => {
                    let state = state.clone();
                    tokio::spawn(async move {
                        let _permit = permit;
                        let spec: Result<CheckSpec, _> = serde_json::from_value(row.get("spec"));
                        if let Ok(spec) = spec {
                            let result = probe(&spec).await;
                            if let Err(error) =
                                finish(&state, &row.get::<String, _>("id"), &lease, &result).await
                            {
                                tracing::error!(%error,"Synthetic result persistence failed");
                            }
                        }
                    });
                }
                Ok(None) => break,
                Err(error) => {
                    tracing::error!(%error,"Synthetic scheduler failed");
                    break;
                }
            }
        }
    }
}

/// Result, incident transition, and notification outbox commit together.
pub async fn finish(state: &AppState, id: &str, lease: &str, result: &ProbeResult) -> Result<()> {
    let db = state.database.as_ref().context("Database unavailable")?;
    let mut tx = db.pool().begin().await?;
    let Some(row) = sqlx::query(
        "SELECT * FROM synthetic_checks WHERE id=$1 AND lease_token=$2 AND enabled FOR UPDATE",
    )
    .bind(id)
    .bind(lease)
    .fetch_optional(&mut *tx)
    .await?
    else {
        return Ok(());
    };
    let owner: i64 = row.get("owner_api_key_id");
    let spec: CheckSpec = serde_json::from_value(row.get("spec"))?;
    let failures = if result.success {
        0
    } else {
        row.get::<i32, _>("consecutive_failures").saturating_add(1)
    };
    let active: Option<String> = row.get("active_alert_id");
    let transition = incident_transition(
        &spec,
        &row.get::<String, _>("agent_id"),
        active.as_deref(),
        failures,
        result,
    );
    let new_active = if let Some(alert) = &transition {
        if alert.state == AlertState::Resolved {
            None
        } else {
            Some(alert.id.clone())
        }
    } else {
        active
    };
    sqlx::query("UPDATE synthetic_checks SET last_result=$2,consecutive_failures=$3,active_alert_id=$4,next_run_at=now()+make_interval(secs=>$5),leased_until=NULL,lease_token=NULL WHERE id=$1")
        .bind(id).bind(serde_json::to_value(result)?).bind(failures).bind(new_active).bind(spec.interval_seconds as f64).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO synthetic_results(check_id,result) VALUES($1,$2)")
        .bind(id)
        .bind(serde_json::to_value(result)?)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM synthetic_results WHERE check_id=$1 AND (checked_at<now()-interval '30 days' OR id<COALESCE((SELECT id FROM synthetic_results WHERE check_id=$1 ORDER BY id DESC OFFSET 9999 LIMIT 1),0))")
        .bind(id).execute(&mut *tx).await?;
    if let Some(alert) = &transition {
        let resolved = alert.state == AlertState::Resolved;
        sqlx::query("INSERT INTO alerts(alert_id,rule_id,rule_name,agent_id,agent_name,severity,message,triggered_at,resolved_at,state,metric,current_value,threshold,condition_type,acknowledged) VALUES($1,$2,$3,$4,$5,'critical',$6,$7,$8,$9,'synthetic_failures',$10,$11,'greater_than',false) ON CONFLICT(alert_id) DO UPDATE SET state=EXCLUDED.state,resolved_at=EXCLUDED.resolved_at,message=EXCLUDED.message,current_value=EXCLUDED.current_value")
            .bind(&alert.id).bind(&alert.rule_id).bind(&alert.rule_name).bind(&alert.agent_id).bind(&alert.agent_name).bind(&alert.message)
            .bind(alert.triggered_at).bind(alert.resolved_at).bind(if resolved {"resolved"} else {"firing"})
            .bind(alert.current_value).bind(alert.threshold).execute(&mut *tx).await?;
        let event = if resolved {
            AlertEventType::Resolved
        } else {
            AlertEventType::Firing
        };
        let payload = alert_payload(&AlertTransition {
            alert: alert.clone(),
            event_type: event.clone(),
            channel_ids: spec.channels.clone(),
        });
        sqlx::query("INSERT INTO notification_deliveries(alert_id,channel_id,channel_name,event_type,status,payload,owner_api_key_id) SELECT $1,c.id,c.name,$2,'pending',$3,c.owner_api_key_id FROM notification_channels c WHERE c.owner_api_key_id=$4 AND c.enabled AND c.id=ANY($5) AND NOT EXISTS(SELECT 1 FROM alert_silences s WHERE s.owner_api_key_id=$4 AND s.starts_at<=now() AND s.ends_at>now() AND (s.agent_id IS NULL OR s.agent_id=$6) AND (s.rule_id IS NULL OR s.rule_id=$7))")
            .bind(&alert.id).bind(event.as_str()).bind(payload).bind(owner).bind(&spec.channels).bind(&alert.agent_id).bind(&alert.rule_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO incident_events(event_id,alert_id,rule_id,agent_id,event_type,severity,title,description,metadata,occurred_at) VALUES($1,$2,$3,$4,$5,'critical',$6,$7,$8,now())")
            .bind(uuid::Uuid::new_v4().to_string()).bind(&alert.id).bind(&alert.rule_id).bind(&alert.agent_id).bind(if resolved {"recovery"} else {"firing"})
            .bind(format!("Synthetic: {}",spec.name)).bind(&alert.message).bind(serde_json::to_value(result)?).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    if let Some(alert) = transition {
        // Read committed fields so an acknowledgement is not overwritten on recovery.
        let persisted = db
            .get_alert(&alert.id)
            .await?
            .context("Committed alert missing")?;
        state
            .alert_manager
            .cache_persisted_alert(persisted.clone())
            .await;
        state
            .ws_manager
            .broadcast_alert(crate::websocket::WsMessage::Alert {
                alert_id: persisted.id,
                agent_id: persisted.agent_id,
                severity: "critical".into(),
                message: persisted.message,
                state: if persisted.state == AlertState::Resolved {
                    "resolved".into()
                } else {
                    "firing".into()
                },
            });
    }
    Ok(())
}

fn incident_transition(
    spec: &CheckSpec,
    agent: &str,
    active: Option<&str>,
    failures: i32,
    result: &ProbeResult,
) -> Option<Alert> {
    let resolved = result.success && active.is_some();
    if !resolved
        && (active.is_some() || result.success || failures < (spec.failure_threshold as i32))
    {
        return None;
    }
    Some(Alert {
        id: active
            .map(str::to_owned)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        rule_id: agent.to_owned(),
        rule_name: spec.name.clone(),
        agent_id: agent.to_owned(),
        agent_name: spec.name.clone(),
        state: if resolved {
            AlertState::Resolved
        } else {
            AlertState::Firing
        },
        metric: "synthetic_failures".into(),
        current_value: f64::from(failures),
        threshold: f64::from(spec.failure_threshold),
        condition: AlertCondition::GreaterThan,
        severity: AlertSeverity::Critical,
        message: if resolved {
            format!("{} is reachable again", spec.name)
        } else {
            format!(
                "{} failed {failures} consecutive checks: {}",
                spec.name,
                result.error.as_deref().unwrap_or("check failed")
            )
        },
        triggered_at: Utc::now(),
        resolved_at: resolved.then(Utc::now),
        acknowledged: false,
        acknowledged_at: None,
        acknowledged_by: None,
        last_notification_at: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires explicit public-network probe permission"]
    async fn public_probe_smoke_test() {
        assert_eq!(
            std::env::var("WATCHTOWER_ENABLE_PUBLIC_PROBES").as_deref(),
            Ok("1")
        );
        for kind in [
            CheckKind::Http,
            CheckKind::Tcp,
            CheckKind::Dns,
            CheckKind::Tls,
        ] {
            let mut value = spec();
            value.kind = kind.clone();
            value.timeout_seconds = 10;
            value.tls_expiry_days = 0;
            value.target = if kind == CheckKind::Http {
                "https://example.com/".into()
            } else {
                "example.com".into()
            };
            if kind == CheckKind::Tcp {
                value.port = Some(443);
            }
            if kind == CheckKind::Http {
                value.expected_content = Some("Example Domain".into());
            }
            value.validate().unwrap();
            let result = probe(&value).await;
            assert!(result.success, "{kind:?} failed: {:?}", result.error);
            assert!(result.response_time_ms > 0.0);
            if kind == CheckKind::Http {
                assert_eq!(result.status_code, Some(200));
                assert_eq!(result.content_matched, Some(true));
            }
            if kind == CheckKind::Tls {
                assert!(result.tls_days_remaining.unwrap() > 0.0);
                assert!(result.tls_expires_at.is_some());
            }
        }
        let mut value = spec();
        value.target = "https://example.com/".into();
        value.timeout_seconds = 10;
        value.expected_content = Some("watchtower-intentionally-absent-content".into());
        let result = probe(&value).await;
        assert!(!result.success);
        assert_eq!(result.content_matched, Some(false));
        assert_eq!(result.status_code, Some(200));
        value.expected_content = None;
        value.expected_status = 503;
        let result = probe(&value).await;
        assert!(!result.success);
        assert_eq!(result.status_code, Some(200));
        value.target = "http://169.254.169.254/latest/meta-data/".into();
        let result = probe(&value).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("public"));
    }
    fn spec() -> CheckSpec {
        serde_json::from_value(
            serde_json::json!({"name":"App","kind":"http","target":"https://example.com"}),
        )
        .unwrap()
    }
    #[test]
    fn validates_bounds_and_destinations() {
        assert!(spec().validate().is_ok());
        for target in [
            "https://127.0.0.1",
            "http://169.254.169.254/latest/meta-data",
            "file:///etc/passwd",
            "https://user:secret@example.com",
        ] {
            let mut value = spec();
            value.target = target.into();
            assert!(value.validate().is_err());
        }
        let mut value = spec();
        value.interval_seconds = 1;
        assert!(value.validate().is_err());
    }
    #[test]
    fn debounces_and_recovers_once() {
        let spec = spec();
        let mut result = ProbeResult {
            checked_at: Utc::now(),
            success: false,
            response_time_ms: 1.0,
            status_code: Some(503),
            content_matched: None,
            resolved_addresses: vec![],
            tls_expires_at: None,
            tls_days_remaining: None,
            error: Some("unavailable".into()),
        };
        assert!(incident_transition(&spec, "synthetic:test", None, 1, &result).is_none());
        assert!(incident_transition(&spec, "synthetic:test", None, 2, &result).is_none());
        let alert = incident_transition(&spec, "synthetic:test", None, 3, &result).unwrap();
        assert!(
            incident_transition(&spec, "synthetic:test", Some(&alert.id), 4, &result).is_none()
        );
        result.success = true;
        let recovery =
            incident_transition(&spec, "synthetic:test", Some(&alert.id), 0, &result).unwrap();
        assert_eq!(recovery.id, alert.id);
        assert_eq!(recovery.state, AlertState::Resolved);
        assert!(incident_transition(&spec, "synthetic:test", None, 0, &result).is_none());
    }
}

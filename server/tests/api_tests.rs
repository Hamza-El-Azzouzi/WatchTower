// Integration tests for the monitoring system API
// These tests use HTTP requests to test the API endpoints as black boxes
// Run with: cargo test --test api_tests

use serde_json::json;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// Helper to start the server for testing
struct TestServer {
    process: Child,
    port: u16,
    _temp_dir: TempDir,
}

impl TestServer {
    fn start() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Use a test port
        let port = 18080;

        // Start server as background process
        let process = Command::new("cargo")
            .args(["run", "--bin", "monitor-server"])
            .env("DATABASE_PATH", db_path.to_str().unwrap())
            .env("SERVER_HOST", "127.0.0.1")
            .env("SERVER_PORT", port.to_string())
            .env("RUST_LOG", "debug")
            .spawn()
            .expect("Failed to start server");

        // Wait for server to be ready
        thread::sleep(Duration::from_secs(2));

        Self {
            process,
            port,
            _temp_dir: temp_dir,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{}", self.port, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}

#[tokio::test]
#[ignore] // Requires server binary to be built - run manually
async fn test_health_check() {
    let server = TestServer::start();

    let client = reqwest::Client::new();
    let response = client
        .get(server.url("/health"))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
}

#[tokio::test]
#[ignore] // Requires server binary to be built - run manually
async fn test_metrics_ingestion() {
    let server = TestServer::start();

    let client = reqwest::Client::new();
    let metric = json!({
        "agent_id": "test-agent",
        "timestamp": 1234567890,
        "metrics": {
            "cpu_usage": 45.5,
            "memory_usage": 60.0
        }
    });

    let response = client
        .post(server.url("/api/metrics"))
        .json(&metric)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);
}

// Note: Most comprehensive integration tests are done with docker-compose
// See the CI pipeline (.github/workflows/ci.yml) for full integration testing
// These tests are kept simple and marked as #[ignore] because they require
// the full server binary to be built and running.

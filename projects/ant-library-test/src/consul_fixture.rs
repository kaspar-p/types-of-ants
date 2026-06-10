use std::{path::PathBuf, time::Duration};

use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    task::JoinHandle,
};
use tracing::{debug, error, info};

pub struct ConsulFixture {
    _consul: tokio::process::Child,
    _data_dir: TempDir,
    handles: Vec<JoinHandle<()>>,
    consul_port: u16,
}

impl Drop for ConsulFixture {
    fn drop(&mut self) {
        info!("Closing consul...");
        let _ = self._consul.kill();
        info!("Dropping read handles");
        for handle in &self.handles {
            handle.abort();
        }
    }
}

impl ConsulFixture {
    pub async fn new() -> Self {
        // Verify the consul binary is present before trying to start the agent,
        // so the error is actionable rather than a bare NotFound from spawn().
        match std::process::Command::new("consul")
            .arg("version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                panic!("consul binary not found in PATH — install it to run tests that use ConsulFixture: https://developer.hashicorp.com/consul/install");
            }
            Err(e) => panic!("consul binary check failed: {e}"),
            Ok(_) => {}
        }

        let http_port = portpicker::pick_unused_port().expect("No ports free: http");
        let gossip_port = portpicker::pick_unused_port().expect("No ports free: gossip");
        let server_port = portpicker::pick_unused_port().expect("No ports free: server");

        let data_dir = tempfile::tempdir().unwrap();

        info!("Binding Consul to port {http_port}");

        let mut cmd = tokio::process::Command::new("consul");
        let cmd = cmd
            .arg("agent")
            .arg("-server")
            .args(["-node", "test-node1"])
            .args([
                "-config-file",
                PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
                    .join("projects")
                    .join("ant-library-test")
                    .join("test-data")
                    .join("consul.hcl")
                    .to_str()
                    .unwrap(),
            ])
            .args(["-http-port", http_port.to_string().as_str()])
            .args(["-serf-lan-port", gossip_port.to_string().as_str()])
            .args(["-server-port", server_port.to_string().as_str()])
            .args(["-bootstrap-expect", "1"])
            .args(["-data-dir", data_dir.path().as_os_str().to_str().unwrap()])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut consul = cmd.spawn().unwrap();

        let stdout = consul.stdout.take().expect("failed to take stdout");
        let stderr = consul.stderr.take().expect("failed to take stderr");
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let stderr_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                debug!("[consul stderr] {}", line);
            }
        });
        let stdout_task = tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                debug!("[consul stdout] {}", line);
            }
        });

        let mut attempts = 0;
        let mut ready = false;
        while !ready && attempts < 100 {
            info!("Waiting for Consul to come up...");
            ready = Self::check_health(http_port).await;

            attempts += 1;
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        if attempts >= 100 {
            panic!("Timed out waiting for consul's healthcheck.")
        }

        info!("Consul started...");

        Self {
            _consul: consul,
            _data_dir: data_dir,
            handles: vec![stdout_task, stderr_task],
            consul_port: http_port,
        }
    }

    async fn check_health(port: u16) -> bool {
        match reqwest::get(format!("http://localhost:{}/v1/agent/self", port))
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(_) => true,
            Err(e) => {
                error!("Failed to get Consul's health: {e}");
                false
            }
        }
    }

    pub fn port(&self) -> u16 {
        self.consul_port
    }

}

use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Child,
    task::JoinHandle,
};

use tracing::debug;

pub fn prefix_log(prefix: &str, child: &mut Child) -> Result<Vec<JoinHandle<()>>, anyhow::Error> {
    let stdout = child.stdout.take().expect("failed to take stdout");
    let stderr = child.stderr.take().expect("failed to take stderr");
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let stdout_task = {
        let prefix = prefix.to_string();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stderr_reader.next_line().await {
                debug!("[{prefix} stdout] {}", line);
            }
        })
    };

    let stderr_task = {
        let prefix = prefix.to_string();
        tokio::spawn(async move {
            while let Ok(Some(line)) = stdout_reader.next_line().await {
                debug!("[{prefix} stderr] {}", line);
            }
        })
    };

    return Ok(vec![stdout_task, stderr_task]);
}

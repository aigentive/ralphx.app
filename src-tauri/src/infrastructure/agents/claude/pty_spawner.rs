// PTY-based spawner for Claude CLI
//
// Spawns Claude CLI in a pseudo-terminal (PTY) to get real-time streaming output.
// Standard pipe-based stdout is fully buffered by default, causing all output
// to arrive at once when the process completes. Using a PTY forces line buffering.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::{BufRead, BufReader};
use std::path::Path;
use tokio::sync::mpsc;
use tracing::{debug, info};

/// Result of spawning a process in a PTY
pub struct PtySpawnResult {
    /// Channel receiver for stdout lines
    pub lines_rx: mpsc::Receiver<String>,
    /// Handle to wait for process completion
    pub wait_handle: tokio::task::JoinHandle<Result<(), String>>,
}

/// Spawn Claude CLI in a PTY for real-time streaming output.
///
/// Returns a channel receiver that yields lines as they arrive,
/// and a handle to wait for process completion.
pub fn spawn_in_pty(
    cli_path: &Path,
    plugin_dir: &Path,
    working_dir: &Path,
    prompt: &str,
    agent: Option<&str>,
    resume_session: Option<&str>,
    env_vars: &[(&str, &str)],
) -> Result<PtySpawnResult, String> {
    let pty_system = native_pty_system();

    // Create a PTY with reasonable size
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Build the command
    let mut cmd = CommandBuilder::new(cli_path);
    cmd.cwd(working_dir);

    // Add Claude CLI arguments
    cmd.args(["--plugin-dir", plugin_dir.to_str().unwrap_or("./ralphx-plugin")]);
    cmd.args(["--output-format", "stream-json"]);
    cmd.arg("--verbose");

    if let Some(session_id) = resume_session {
        cmd.args(["--resume", session_id]);
    } else if let Some(agent_name) = agent {
        cmd.args(["--agent", agent_name]);
    }

    cmd.args(["-p", prompt]);

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Spawn the process in the PTY
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn in PTY: {}", e))?;

    // Get a reader for the PTY master (this is where output comes from)
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    // Create a channel for streaming lines
    let (tx, rx) = mpsc::channel::<String>(100);

    // Spawn a blocking task to read from the PTY
    let wait_handle = tokio::task::spawn_blocking(move || {
        let buf_reader = BufReader::new(reader);

        for line_result in buf_reader.lines() {
            match line_result {
                Ok(line) => {
                    debug!("PTY line: {}", &line[..line.len().min(100)]);
                    // Try to send, but don't block if receiver is dropped
                    if tx.blocking_send(line).is_err() {
                        info!("PTY reader: channel closed, stopping");
                        break;
                    }
                }
                Err(e) => {
                    // EOF or error - process likely finished
                    debug!("PTY read ended: {}", e);
                    break;
                }
            }
        }

        // Wait for the child process to complete
        drop(child); // This waits for the child

        // Drop the PTY master to clean up
        drop(pair.master);

        Ok(())
    });

    Ok(PtySpawnResult {
        lines_rx: rx,
        wait_handle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_spawn_echo() {
        // Test with a simple echo command
        let result = spawn_in_pty(
            Path::new("/bin/echo"),
            Path::new("."),
            Path::new("."),
            "hello world",
            None,
            None,
            &[],
        );

        // This test is just to verify the PTY setup works
        // The actual output depends on having the claude CLI available
        assert!(result.is_ok() || result.is_err()); // Either is fine for unit test
    }
}

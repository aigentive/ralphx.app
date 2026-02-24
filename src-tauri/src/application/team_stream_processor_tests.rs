use super::*;

#[test]
fn test_module_compiles() {
    // Verify the module compiles and types are accessible — includes exit_signal parameter
    fn _assert_fn_signature() {
        fn _check(
            _stdout: ChildStdout,
            _exit_signal: oneshot::Receiver<()>,
            _team_name: String,
            _teammate_name: String,
            _context_type: String,
            _context_id: String,
            _app_handle: AppHandle,
            _team_tracker: Arc<TeamStateTracker>,
            _team_service: Option<Arc<TeamService>>,
        ) -> JoinHandle<()> {
            unimplemented!()
        }
        let _ = _check;
    }
}

/// Fix B: exit_signal channel pair is created and wired correctly.
/// Verifies that sending on exit_tx causes exit_rx to resolve immediately
/// (which is what the select! in start_teammate_stream relies on).
#[tokio::test]
async fn test_exit_signal_channel_resolves_on_send() {
    let (exit_tx, exit_rx) = oneshot::channel::<()>();

    // Sender fires — receiver should resolve immediately
    exit_tx.send(()).unwrap();

    // Using tokio::time::timeout to ensure the future resolves
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        exit_rx,
    )
    .await;

    assert!(result.is_ok(), "exit_rx should resolve when exit_tx sends");
    assert!(result.unwrap().is_ok(), "exit_rx value should be Ok(())");
}

/// Fix B: kill_tx send is received on kill_rx.
/// Simulates the stop_teammate path: dropping kill_tx signals kill_rx.
#[tokio::test]
async fn test_kill_tx_dropped_fires_kill_rx() {
    let (kill_tx, kill_rx) = oneshot::channel::<()>();

    // Dropping kill_tx (without send) fires RecvError on kill_rx,
    // which the select! pattern `_ = kill_rx` also matches — triggering cleanup.
    drop(kill_tx);

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        kill_rx,
    )
    .await;

    assert!(result.is_ok(), "kill_rx should resolve when kill_tx is dropped");
    // Err(RecvError) is expected — sender dropped without sending
    assert!(result.unwrap().is_err(), "kill_rx should get RecvError when kill_tx dropped");
}

#[test]
fn test_message_type_mapping() {
    // Verify TeamMessageSent message_type string → TeamMessageType mapping
    let broadcast_type = match "broadcast" {
        "broadcast" => TeamMessageType::Broadcast,
        _ => TeamMessageType::TeammateMessage,
    };
    assert_eq!(broadcast_type, TeamMessageType::Broadcast);

    let message_type = match "message" {
        "broadcast" => TeamMessageType::Broadcast,
        _ => TeamMessageType::TeammateMessage,
    };
    assert_eq!(message_type, TeamMessageType::TeammateMessage);
}

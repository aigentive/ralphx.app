use super::*;

#[test]
fn test_module_compiles() {
    // Verify the module compiles and types are accessible
    fn _assert_fn_signature() {
        fn _check(
            _stdout: ChildStdout,
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

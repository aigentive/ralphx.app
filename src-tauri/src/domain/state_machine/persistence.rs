// State-local data persistence helpers
// Provides serialization/deserialization for state data that needs to persist

use super::machine::State;
use super::types::{FailedData, QaFailedData};
use serde::{Deserialize, Serialize};

/// State data container for persistence
///
/// When a state has local data (like QaFailed or Failed), this struct
/// holds the serialized form along with the state type identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateData {
    /// The state type identifier (e.g., "qa_failed", "failed")
    pub state_type: String,
    /// The JSON-serialized state-local data
    pub data: String,
}

impl StateData {
    /// Creates a new StateData instance
    pub fn new(state_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self {
            state_type: state_type.into(),
            data: data.into(),
        }
    }

    /// Extracts state data from a State if it has local data.
    ///
    /// Returns None for states without local data.
    /// Returns Some(StateData) for QaFailed and Failed states.
    pub fn from_state(state: &State) -> Option<Self> {
        match state {
            State::QaFailed(data) => {
                let json = serde_json::to_string(data).ok()?;
                Some(Self::new("qa_failed", json))
            }
            State::Failed(data) => {
                let json = serde_json::to_string(data).ok()?;
                Some(Self::new("failed", json))
            }
            // All other states don't have local data
            _ => None,
        }
    }

    /// Reconstructs a State with its local data.
    ///
    /// Takes a state type string and the JSON data, returning the
    /// complete State with deserialized data.
    ///
    /// Returns None if:
    /// - The state_type is not "qa_failed" or "failed"
    /// - The JSON data fails to deserialize
    pub fn into_state(self) -> Option<State> {
        match self.state_type.as_str() {
            "qa_failed" => {
                let data: QaFailedData = serde_json::from_str(&self.data).ok()?;
                Some(State::QaFailed(data))
            }
            "failed" => {
                let data: FailedData = serde_json::from_str(&self.data).ok()?;
                Some(State::Failed(data))
            }
            _ => None,
        }
    }

    /// Loads state-local data and applies it to a parsed State.
    ///
    /// If the state has local data (QaFailed, Failed), this replaces
    /// the default data with the persisted data. Otherwise returns
    /// the state unchanged.
    pub fn apply_to_state(self, state: State) -> State {
        // Only replace data for states that match the data type
        match (state, self.state_type.as_str()) {
            (State::QaFailed(_), "qa_failed") => {
                if let Ok(data) = serde_json::from_str::<QaFailedData>(&self.data) {
                    State::QaFailed(data)
                } else {
                    State::QaFailed(QaFailedData::default())
                }
            }
            (State::Failed(_), "failed") => {
                if let Ok(data) = serde_json::from_str::<FailedData>(&self.data) {
                    State::Failed(data)
                } else {
                    State::Failed(FailedData::default())
                }
            }
            // State type mismatch or state doesn't have data - return as-is
            (state, _) => state,
        }
    }
}

/// Checks if a state has local data that needs persistence.
pub fn state_has_data(state: &State) -> bool {
    matches!(state, State::QaFailed(_) | State::Failed(_))
}

/// Serializes QaFailedData to JSON string.
pub fn serialize_qa_failed_data(data: &QaFailedData) -> Result<String, serde_json::Error> {
    serde_json::to_string(data)
}

/// Deserializes QaFailedData from JSON string.
pub fn deserialize_qa_failed_data(json: &str) -> Result<QaFailedData, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serializes FailedData to JSON string.
pub fn serialize_failed_data(data: &FailedData) -> Result<String, serde_json::Error> {
    serde_json::to_string(data)
}

/// Deserializes FailedData from JSON string.
pub fn deserialize_failed_data(json: &str) -> Result<FailedData, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::state_machine::types::QaFailure;

    // ==================
    // StateData tests
    // ==================

    #[test]
    fn test_state_data_new() {
        let sd = StateData::new("qa_failed", r#"{"failures":[]}"#);
        assert_eq!(sd.state_type, "qa_failed");
        assert_eq!(sd.data, r#"{"failures":[]}"#);
    }

    #[test]
    fn test_state_data_from_state_returns_none_for_simple_states() {
        assert!(StateData::from_state(&State::Backlog).is_none());
        assert!(StateData::from_state(&State::Ready).is_none());
        assert!(StateData::from_state(&State::Blocked).is_none());
        assert!(StateData::from_state(&State::Executing).is_none());
        assert!(StateData::from_state(&State::ReExecuting).is_none());
        assert!(StateData::from_state(&State::QaRefining).is_none());
        assert!(StateData::from_state(&State::QaTesting).is_none());
        assert!(StateData::from_state(&State::QaPassed).is_none());
        assert!(StateData::from_state(&State::PendingReview).is_none());
        assert!(StateData::from_state(&State::Reviewing).is_none());
        assert!(StateData::from_state(&State::ReviewPassed).is_none());
        assert!(StateData::from_state(&State::RevisionNeeded).is_none());
        assert!(StateData::from_state(&State::Approved).is_none());
        assert!(StateData::from_state(&State::Cancelled).is_none());
    }

    #[test]
    fn test_state_data_from_state_qa_failed() {
        let qa_data = QaFailedData::single(QaFailure::new("test_foo", "assertion failed"));
        let state = State::QaFailed(qa_data.clone());

        let state_data = StateData::from_state(&state)
            .expect("Failed to extract state data for qa_failed");
        assert_eq!(state_data.state_type, "qa_failed");
        assert!(state_data.data.contains("test_foo"));
        assert!(state_data.data.contains("assertion failed"));
    }

    #[test]
    fn test_state_data_from_state_failed() {
        let failed_data = FailedData::new("Build error");
        let state = State::Failed(failed_data);

        let state_data = StateData::from_state(&state)
            .expect("Failed to extract state data for failed");
        assert_eq!(state_data.state_type, "failed");
        assert!(state_data.data.contains("Build error"));
    }

    #[test]
    fn test_state_data_into_state_qa_failed() {
        let qa_data = QaFailedData::single(QaFailure::new("test_bar", "expected 1, got 2"));
        let json = serde_json::to_string(&qa_data)
            .expect("Failed to serialize QaFailedData");
        let state_data = StateData::new("qa_failed", json);

        let state = state_data.into_state()
            .expect("Failed to deserialize qa_failed state");
        if let State::QaFailed(data) = state {
            assert!(data.has_failures());
            assert_eq!(data.first_error(), Some("expected 1, got 2"));
        } else {
            panic!("Expected QaFailed state");
        }
    }

    #[test]
    fn test_state_data_into_state_failed() {
        let failed_data = FailedData::new("Timeout").with_details("Command took 60s");
        let json = serde_json::to_string(&failed_data)
            .expect("Failed to serialize FailedData");
        let state_data = StateData::new("failed", json);

        let state = state_data.into_state()
            .expect("Failed to deserialize failed state");
        if let State::Failed(data) = state {
            assert_eq!(data.error, "Timeout");
            assert_eq!(data.details, Some("Command took 60s".to_string()));
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_state_data_into_state_unknown_type() {
        let state_data = StateData::new("unknown", "{}");
        assert!(state_data.into_state().is_none());
    }

    #[test]
    fn test_state_data_into_state_invalid_json() {
        let state_data = StateData::new("qa_failed", "not valid json");
        assert!(state_data.into_state().is_none());
    }

    #[test]
    fn test_state_data_roundtrip_qa_failed() {
        let original_data = QaFailedData::single(QaFailure::new("test_roundtrip", "failed"));
        let original_state = State::QaFailed(original_data.clone());

        let state_data = StateData::from_state(&original_state)
            .expect("Failed to extract state data in roundtrip");
        let restored_state = state_data.into_state()
            .expect("Failed to restore state from data in roundtrip");

        if let State::QaFailed(restored_data) = restored_state {
            assert_eq!(original_data.failures.len(), restored_data.failures.len());
            assert_eq!(original_data.first_error(), restored_data.first_error());
        } else {
            panic!("Expected QaFailed state");
        }
    }

    #[test]
    fn test_state_data_roundtrip_failed() {
        let original_data = FailedData::timeout("timeout error");
        let original_state = State::Failed(original_data.clone());

        let state_data = StateData::from_state(&original_state)
            .expect("Failed to extract failed state data in roundtrip");
        let restored_state = state_data.into_state()
            .expect("Failed to restore failed state from data in roundtrip");

        if let State::Failed(restored_data) = restored_state {
            assert_eq!(original_data.error, restored_data.error);
            assert_eq!(original_data.is_timeout, restored_data.is_timeout);
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_state_data_apply_to_state_replaces_qa_failed_data() {
        let persisted_data = QaFailedData::single(QaFailure::new("persisted_test", "persisted error"));
        let json = serde_json::to_string(&persisted_data).unwrap();
        let state_data = StateData::new("qa_failed", json);

        // Parse state from string (gets default data)
        let parsed_state = State::QaFailed(QaFailedData::default());

        // Apply persisted data
        let restored = state_data.apply_to_state(parsed_state);

        if let State::QaFailed(data) = restored {
            assert!(data.has_failures());
            assert_eq!(data.first_error(), Some("persisted error"));
        } else {
            panic!("Expected QaFailed state");
        }
    }

    #[test]
    fn test_state_data_apply_to_state_replaces_failed_data() {
        let persisted_data = FailedData::new("persisted error").with_details("stack trace");
        let json = serde_json::to_string(&persisted_data).unwrap();
        let state_data = StateData::new("failed", json);

        // Parse state from string (gets default data)
        let parsed_state = State::Failed(FailedData::default());

        // Apply persisted data
        let restored = state_data.apply_to_state(parsed_state);

        if let State::Failed(data) = restored {
            assert_eq!(data.error, "persisted error");
            assert_eq!(data.details, Some("stack trace".to_string()));
        } else {
            panic!("Expected Failed state");
        }
    }

    #[test]
    fn test_state_data_apply_to_state_ignores_type_mismatch() {
        // StateData for qa_failed but state is failed
        let qa_data = QaFailedData::single(QaFailure::new("test", "error"));
        let json = serde_json::to_string(&qa_data).unwrap();
        let state_data = StateData::new("qa_failed", json);

        let parsed_state = State::Failed(FailedData::new("original"));
        let result = state_data.apply_to_state(parsed_state);

        // Should keep original state unchanged
        if let State::Failed(data) = result {
            assert_eq!(data.error, "original");
        } else {
            panic!("Expected Failed state unchanged");
        }
    }

    #[test]
    fn test_state_data_apply_to_state_ignores_non_data_states() {
        let state_data = StateData::new("qa_failed", "{}");

        // Apply to state without data
        let parsed_state = State::Ready;
        let result = state_data.apply_to_state(parsed_state);

        assert_eq!(result, State::Ready);
    }

    #[test]
    fn test_state_data_apply_to_state_handles_invalid_json() {
        let state_data = StateData::new("qa_failed", "invalid json");

        let parsed_state = State::QaFailed(QaFailedData::default());
        let result = state_data.apply_to_state(parsed_state);

        // Should return state with default data
        if let State::QaFailed(data) = result {
            assert!(!data.has_failures());
        } else {
            panic!("Expected QaFailed state");
        }
    }

    // ==================
    // Helper function tests
    // ==================

    #[test]
    fn test_state_has_data_returns_true_for_qa_failed() {
        assert!(state_has_data(&State::QaFailed(QaFailedData::default())));
    }

    #[test]
    fn test_state_has_data_returns_true_for_failed() {
        assert!(state_has_data(&State::Failed(FailedData::default())));
    }

    #[test]
    fn test_state_has_data_returns_false_for_other_states() {
        assert!(!state_has_data(&State::Backlog));
        assert!(!state_has_data(&State::Ready));
        assert!(!state_has_data(&State::Blocked));
        assert!(!state_has_data(&State::Executing));
        assert!(!state_has_data(&State::ReExecuting));
        assert!(!state_has_data(&State::QaRefining));
        assert!(!state_has_data(&State::QaTesting));
        assert!(!state_has_data(&State::QaPassed));
        assert!(!state_has_data(&State::PendingReview));
        assert!(!state_has_data(&State::Reviewing));
        assert!(!state_has_data(&State::ReviewPassed));
        assert!(!state_has_data(&State::RevisionNeeded));
        assert!(!state_has_data(&State::Approved));
        assert!(!state_has_data(&State::Cancelled));
    }

    #[test]
    fn test_serialize_qa_failed_data() {
        let data = QaFailedData::single(QaFailure::new("test_serialize", "error"));
        let json = serialize_qa_failed_data(&data).unwrap();
        assert!(json.contains("test_serialize"));
        assert!(json.contains("error"));
    }

    #[test]
    fn test_deserialize_qa_failed_data() {
        let json = r#"{"failures":[{"test_name":"test_deser","error":"fail"}],"retry_count":2,"notified":false}"#;
        let data = deserialize_qa_failed_data(json).unwrap();
        assert!(data.has_failures());
        assert_eq!(data.retry_count, 2);
    }

    #[test]
    fn test_deserialize_qa_failed_data_invalid() {
        let result = deserialize_qa_failed_data("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_failed_data() {
        let data = FailedData::new("test error").with_details("details here");
        let json = serialize_failed_data(&data).unwrap();
        assert!(json.contains("test error"));
        assert!(json.contains("details here"));
    }

    #[test]
    fn test_deserialize_failed_data() {
        let json = r#"{"error":"deser error","details":"trace","is_timeout":true,"notified":false}"#;
        let data = deserialize_failed_data(json).unwrap();
        assert_eq!(data.error, "deser error");
        assert!(data.is_timeout);
    }

    #[test]
    fn test_deserialize_failed_data_invalid() {
        let result = deserialize_failed_data("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_qa_failed() {
        let mut original = QaFailedData::single(QaFailure::new("roundtrip", "test"));
        original.retry_count = 5;
        let json = serialize_qa_failed_data(&original).unwrap();
        let restored = deserialize_qa_failed_data(&json).unwrap();

        assert_eq!(original.retry_count, restored.retry_count);
        assert_eq!(original.failures.len(), restored.failures.len());
    }

    #[test]
    fn test_serialize_deserialize_roundtrip_failed() {
        let original = FailedData::timeout("timeout error").with_details("command timed out");
        let json = serialize_failed_data(&original).unwrap();
        let restored = deserialize_failed_data(&json).unwrap();

        assert_eq!(original.error, restored.error);
        assert_eq!(original.is_timeout, restored.is_timeout);
        assert_eq!(original.details, restored.details);
    }

    // ==================
    // StateData Serialize/Deserialize tests
    // ==================

    #[test]
    fn test_state_data_serializes() {
        let sd = StateData::new("qa_failed", r#"{"failures":[]}"#);
        let json = serde_json::to_string(&sd).unwrap();
        assert!(json.contains("qa_failed"));
    }

    #[test]
    fn test_state_data_deserializes() {
        let json = r#"{"state_type":"failed","data":"{\"error\":\"test\"}"}"#;
        let sd: StateData = serde_json::from_str(json).unwrap();
        assert_eq!(sd.state_type, "failed");
    }

    #[test]
    fn test_state_data_clone() {
        let original = StateData::new("qa_failed", "{}");
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}

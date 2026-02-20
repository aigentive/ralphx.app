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
#[path = "persistence_tests.rs"]
mod tests;

// Pattern detection for supervisor
// Detects infinite loops, stuck agents, and poor task definitions

use super::events::ToolCallInfo;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Types of patterns that can be detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pattern {
    /// Same tool call repeated multiple times
    InfiniteLoop,
    /// No progress for extended period
    Stuck,
    /// Agent requesting clarification repeatedly
    PoorTaskDefinition,
    /// Same error repeating
    RepeatingError,
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::InfiniteLoop => write!(f, "Infinite loop detected"),
            Pattern::Stuck => write!(f, "Agent appears stuck"),
            Pattern::PoorTaskDefinition => write!(f, "Poor task definition"),
            Pattern::RepeatingError => write!(f, "Repeating error"),
        }
    }
}

/// Result of pattern detection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionResult {
    /// The pattern detected
    pub pattern: Pattern,
    /// Confidence level (0-100)
    pub confidence: u8,
    /// Description of what was detected
    pub description: String,
    /// Number of occurrences (for loops/repeating errors)
    pub occurrences: usize,
}

impl DetectionResult {
    pub fn new(
        pattern: Pattern,
        confidence: u8,
        description: impl Into<String>,
        occurrences: usize,
    ) -> Self {
        Self {
            pattern,
            confidence: confidence.min(100),
            description: description.into(),
            occurrences,
        }
    }

    /// Check if this is a high-confidence detection
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 80
    }
}

/// Rolling window of recent tool calls for pattern detection
#[derive(Debug, Clone)]
pub struct ToolCallWindow {
    /// Maximum size of the window
    max_size: usize,
    /// Recent tool calls
    calls: VecDeque<ToolCallInfo>,
    /// Threshold for loop detection
    loop_threshold: usize,
    /// Track stuck state
    no_progress_count: usize,
}

impl Default for ToolCallWindow {
    fn default() -> Self {
        Self::new(10, 3)
    }
}

impl ToolCallWindow {
    /// Create a new window with specified size and loop threshold
    pub fn new(max_size: usize, loop_threshold: usize) -> Self {
        Self {
            max_size,
            calls: VecDeque::with_capacity(max_size),
            loop_threshold,
            no_progress_count: 0,
        }
    }

    /// Add a tool call to the window
    pub fn push(&mut self, call: ToolCallInfo) {
        if self.calls.len() >= self.max_size {
            self.calls.pop_front();
        }
        self.calls.push_back(call);
    }

    /// Clear all calls from the window
    pub fn clear(&mut self) {
        self.calls.clear();
        self.no_progress_count = 0;
    }

    /// Get the number of calls in the window
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// Check if the window is empty
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// Record no progress (called on progress tick)
    pub fn record_no_progress(&mut self) {
        self.no_progress_count += 1;
    }

    /// Record progress (resets the no-progress counter)
    pub fn record_progress(&mut self) {
        self.no_progress_count = 0;
    }

    /// Detect infinite loop pattern
    /// Returns Some if the same call appears >= threshold times
    pub fn detect_loop(&self) -> Option<DetectionResult> {
        if self.calls.len() < self.loop_threshold {
            return None;
        }

        // Count occurrences of each call
        let mut max_count = 0;
        let mut max_call: Option<&ToolCallInfo> = None;

        for (i, call) in self.calls.iter().enumerate() {
            let count = self
                .calls
                .iter()
                .skip(i)
                .filter(|c| call.is_similar_to(c))
                .count();
            if count > max_count {
                max_count = count;
                max_call = Some(call);
            }
        }

        if max_count >= self.loop_threshold {
            if let Some(call) = max_call {
                let confidence = if max_count > self.loop_threshold + 1 {
                    95
                } else if max_count > self.loop_threshold {
                    85
                } else {
                    75
                };

                Some(DetectionResult::new(
                    Pattern::InfiniteLoop,
                    confidence,
                    format!(
                        "Tool '{}' called {} times with similar arguments",
                        call.tool_name, max_count
                    ),
                    max_count,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Detect stuck pattern
    /// Returns Some if no progress for threshold minutes
    pub fn detect_stuck(&self, no_progress_threshold: usize) -> Option<DetectionResult> {
        if self.no_progress_count >= no_progress_threshold {
            let confidence = if self.no_progress_count > no_progress_threshold + 2 {
                95
            } else if self.no_progress_count > no_progress_threshold {
                80
            } else {
                70
            };

            Some(DetectionResult::new(
                Pattern::Stuck,
                confidence,
                format!(
                    "No progress for {} consecutive checks",
                    self.no_progress_count
                ),
                self.no_progress_count,
            ))
        } else {
            None
        }
    }

    /// Detect repeating error pattern
    pub fn detect_repeating_error(&self) -> Option<DetectionResult> {
        // Count failed calls with same error
        let failed_calls: Vec<_> = self.calls.iter().filter(|c| !c.success).collect();

        if failed_calls.len() < 2 {
            return None;
        }

        // Group by error message
        let mut max_count = 0;
        let mut max_error: Option<&str> = None;

        for call in &failed_calls {
            if let Some(ref error) = call.error {
                let count = failed_calls
                    .iter()
                    .filter(|c| c.error.as_deref() == Some(error.as_str()))
                    .count();
                if count > max_count {
                    max_count = count;
                    max_error = Some(error.as_str());
                }
            }
        }

        if max_count >= 2 {
            let confidence = if max_count >= 4 {
                90
            } else if max_count >= 3 {
                80
            } else {
                70
            };
            Some(DetectionResult::new(
                Pattern::RepeatingError,
                confidence,
                format!(
                    "Error '{}' occurred {} times",
                    max_error.unwrap_or("unknown"),
                    max_count
                ),
                max_count,
            ))
        } else {
            None
        }
    }

    /// Run all detection patterns and return any matches
    pub fn detect_all(&self, no_progress_threshold: usize) -> Vec<DetectionResult> {
        let mut results = Vec::new();

        if let Some(r) = self.detect_loop() {
            results.push(r);
        }
        if let Some(r) = self.detect_stuck(no_progress_threshold) {
            results.push(r);
        }
        if let Some(r) = self.detect_repeating_error() {
            results.push(r);
        }

        results
    }
}

/// Detect poor task definition based on agent behavior
pub fn detect_poor_task_definition(clarification_requests: usize) -> Option<DetectionResult> {
    if clarification_requests >= 3 {
        let confidence = if clarification_requests >= 5 {
            90
        } else if clarification_requests >= 4 {
            80
        } else {
            70
        };
        Some(DetectionResult::new(
            Pattern::PoorTaskDefinition,
            confidence,
            format!(
                "Agent requested clarification {} times",
                clarification_requests
            ),
            clarification_requests,
        ))
    } else {
        None
    }
}

#[cfg(test)]
#[path = "patterns_tests.rs"]
mod tests;

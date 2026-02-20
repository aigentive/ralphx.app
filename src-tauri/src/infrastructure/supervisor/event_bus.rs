// EventBus for supervisor
// Uses tokio::broadcast for event pub/sub

use crate::domain::supervisor::SupervisorEvent;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Default channel capacity for event bus
const DEFAULT_CAPACITY: usize = 256;

/// Event bus for supervisor events
/// Uses tokio::broadcast channel for multi-subscriber support
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SupervisorEvent>,
    events_published: Arc<AtomicU64>,
}

impl EventBus {
    /// Create a new EventBus with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new EventBus with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            events_published: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Publish an event to all subscribers
    /// Returns Ok(count) with the number of receivers that received the event
    /// Returns Err if there are no active subscribers
    pub fn publish(&self, event: SupervisorEvent) -> Result<usize, SupervisorEvent> {
        match self.sender.send(event) {
            Ok(count) => {
                self.events_published.fetch_add(1, Ordering::Relaxed);
                Ok(count)
            }
            Err(broadcast::error::SendError(event)) => Err(event),
        }
    }

    /// Subscribe to events
    /// Returns an EventSubscriber that can be used to receive events
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Get the total number of events published
    pub fn events_published(&self) -> u64 {
        self.events_published.load(Ordering::Relaxed)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscriber for receiving supervisor events
pub struct EventSubscriber {
    receiver: broadcast::Receiver<SupervisorEvent>,
}

impl EventSubscriber {
    /// Try to receive the next event without blocking
    /// Returns Ok(event) if an event is available
    /// Returns Err(TryRecvError::Empty) if no event is available
    /// Returns Err(TryRecvError::Lagged(count)) if messages were missed
    /// Returns Err(TryRecvError::Closed) if the channel is closed
    pub fn try_recv(&mut self) -> Result<SupervisorEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive the next event, blocking until one is available
    /// Returns Ok(event) when an event is received
    /// Returns Err(RecvError::Lagged(count)) if messages were missed
    /// Returns Err(RecvError::Closed) if the channel is closed
    pub async fn recv(&mut self) -> Result<SupervisorEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.receiver.is_empty() && self.receiver.is_empty()
    }
}

#[cfg(test)]
#[path = "event_bus_tests.rs"]
mod tests;

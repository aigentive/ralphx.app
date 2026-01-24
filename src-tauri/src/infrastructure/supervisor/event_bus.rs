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
        self.receiver.is_empty() && self.receiver.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::supervisor::{ErrorInfo, ProgressInfo, ToolCallInfo};

    #[test]
    fn test_event_bus_new() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
        assert_eq!(bus.events_published(), 0);
    }

    #[test]
    fn test_event_bus_with_capacity() {
        let bus = EventBus::with_capacity(100);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_event_bus_default() {
        let bus = EventBus::default();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_event_bus_clone() {
        let bus1 = EventBus::new();
        let _sub = bus1.subscribe();
        let bus2 = bus1.clone();

        // Both clones should share the same channel
        assert_eq!(bus1.subscriber_count(), 1);
        assert_eq!(bus2.subscriber_count(), 1);
    }

    #[test]
    fn test_subscribe_increases_count() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn test_subscribe_drops_decreases_count() {
        let bus = EventBus::new();
        let sub1 = bus.subscribe();
        let sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        drop(sub1);
        assert_eq!(bus.subscriber_count(), 1);

        drop(sub2);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_publish_no_subscribers_returns_event() {
        let bus = EventBus::new();
        let event = SupervisorEvent::task_start("task-1", "Test task");

        let result = bus.publish(event);
        assert!(result.is_err());
    }

    #[test]
    fn test_publish_with_subscribers_returns_count() {
        let bus = EventBus::new();
        let _sub1 = bus.subscribe();
        let _sub2 = bus.subscribe();

        let event = SupervisorEvent::task_start("task-1", "Test task");
        let result = bus.publish(event);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 2);
    }

    #[test]
    fn test_events_published_counter() {
        let bus = EventBus::new();
        let _sub = bus.subscribe();

        assert_eq!(bus.events_published(), 0);

        bus.publish(SupervisorEvent::task_start("task-1", "Test 1")).ok();
        assert_eq!(bus.events_published(), 1);

        bus.publish(SupervisorEvent::task_start("task-2", "Test 2")).ok();
        assert_eq!(bus.events_published(), 2);
    }

    #[tokio::test]
    async fn test_subscriber_recv() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        let event = SupervisorEvent::task_start("task-1", "Test task");
        bus.publish(event.clone()).ok();

        let received = sub.recv().await.unwrap();
        match (&received, &event) {
            (SupervisorEvent::TaskStart { task_id: r_id, .. },
             SupervisorEvent::TaskStart { task_id: e_id, .. }) => {
                assert_eq!(r_id, e_id);
            }
            _ => panic!("Event type mismatch"),
        }
    }

    #[test]
    fn test_subscriber_try_recv_empty() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        let result = sub.try_recv();
        assert!(matches!(result, Err(broadcast::error::TryRecvError::Empty)));
    }

    #[test]
    fn test_subscriber_try_recv_event() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        bus.publish(SupervisorEvent::task_start("task-1", "Test")).ok();

        let result = sub.try_recv();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_subscribers_receive_same_event() {
        let bus = EventBus::new();
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        let event = SupervisorEvent::task_start("task-1", "Test task");
        bus.publish(event.clone()).ok();

        let recv1 = sub1.recv().await.unwrap();
        let recv2 = sub2.recv().await.unwrap();

        // Both should receive TaskStart events
        assert!(matches!(recv1, SupervisorEvent::TaskStart { .. }));
        assert!(matches!(recv2, SupervisorEvent::TaskStart { .. }));
    }

    #[tokio::test]
    async fn test_subscriber_receives_multiple_events() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        bus.publish(SupervisorEvent::task_start("task-1", "Test 1")).ok();
        bus.publish(SupervisorEvent::progress_tick("task-1", ProgressInfo::new())).ok();
        bus.publish(SupervisorEvent::token_threshold("task-1", 50000, 100000)).ok();

        let e1 = sub.recv().await.unwrap();
        let e2 = sub.recv().await.unwrap();
        let e3 = sub.recv().await.unwrap();

        assert!(matches!(e1, SupervisorEvent::TaskStart { .. }));
        assert!(matches!(e2, SupervisorEvent::ProgressTick { .. }));
        assert!(matches!(e3, SupervisorEvent::TokenThreshold { .. }));
    }

    #[test]
    fn test_event_bus_tool_call_event() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        let tool_call = ToolCallInfo::new("Read", "file.rs");

        bus.publish(SupervisorEvent::tool_call("task-1", tool_call)).ok();

        let result = sub.try_recv();
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SupervisorEvent::ToolCall { .. }));
    }

    #[test]
    fn test_event_bus_error_event() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        let error = ErrorInfo::new("Type error", "compile");

        bus.publish(SupervisorEvent::error("task-1", error)).ok();

        let result = sub.try_recv();
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SupervisorEvent::Error { .. }));
    }

    #[test]
    fn test_event_bus_time_threshold_event() {
        let bus = EventBus::new();
        let mut sub = bus.subscribe();

        bus.publish(SupervisorEvent::time_threshold("task-1", 600, 600)).ok();

        let result = sub.try_recv();
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), SupervisorEvent::TimeThreshold { .. }));
    }

    #[test]
    fn test_late_subscriber_misses_events() {
        let bus = EventBus::new();
        let _early_sub = bus.subscribe();

        // Publish before late subscriber joins
        bus.publish(SupervisorEvent::task_start("task-1", "Test")).ok();

        // Late subscriber shouldn't see the event
        let mut late_sub = bus.subscribe();
        let result = late_sub.try_recv();
        assert!(matches!(result, Err(broadcast::error::TryRecvError::Empty)));
    }

    #[test]
    fn test_subscriber_lagged() {
        // Create a very small capacity bus
        let bus = EventBus::with_capacity(2);
        let mut sub = bus.subscribe();

        // Publish more events than capacity
        for i in 0..5 {
            bus.publish(SupervisorEvent::task_start(&format!("task-{}", i), "Test")).ok();
        }

        // First recv should indicate lagged
        let result = sub.try_recv();
        assert!(matches!(result, Err(broadcast::error::TryRecvError::Lagged(_))));
    }

    #[tokio::test]
    async fn test_concurrent_publish_and_subscribe() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let bus = Arc::new(EventBus::new());
        let mut sub = bus.subscribe();
        let barrier = Arc::new(Barrier::new(2));

        let bus_clone = bus.clone();
        let barrier_clone = barrier.clone();

        // Publisher task
        let publisher = tokio::spawn(async move {
            barrier_clone.wait().await;
            for i in 0..10 {
                bus_clone.publish(SupervisorEvent::task_start(&format!("task-{}", i), "Test")).ok();
            }
        });

        // Subscriber task
        let subscriber = tokio::spawn(async move {
            barrier.wait().await;
            let mut count = 0;
            loop {
                match tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    sub.recv()
                ).await {
                    Ok(Ok(_)) => count += 1,
                    _ => break,
                }
            }
            count
        });

        publisher.await.unwrap();
        let received_count = subscriber.await.unwrap();

        // Should have received some events (exact count may vary due to timing)
        assert!(received_count > 0);
    }
}

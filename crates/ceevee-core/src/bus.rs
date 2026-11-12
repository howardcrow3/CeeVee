use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Events published by feature crates. Variants are added as features land.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// A job row changed state or progress (`job` table).
    JobUpdated {
        #[specta(type = u32)] // specta forbids i64; ids stay far below 2^32
        job_id: i64,
    },
    /// A setting changed.
    SettingChanged { key: String },
}

/// In-process publish/subscribe channel.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event. Returns the number of subscribers that received it.
    pub fn publish(&self, event: Event) -> usize {
        self.tx.send(event).unwrap_or(0)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn subscribers_receive_published_events() {
        let bus = EventBus::new(8);
        let mut rx = bus.subscribe();
        assert_eq!(bus.publish(Event::JobUpdated { job_id: 7 }), 1);
        assert_eq!(rx.recv().await.unwrap(), Event::JobUpdated { job_id: 7 });
    }

    #[test]
    fn publishing_without_subscribers_is_fine() {
        assert_eq!(
            EventBus::new(1).publish(Event::SettingChanged { key: "x".into() }),
            0
        );
    }
}

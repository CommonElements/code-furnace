use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub source: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Event {
    pub fn new(event_type: impl Into<String>, source: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: event_type.into(),
            source: source.into(),
            data,
            timestamp: chrono::Utc::now(),
        }
    }
}

pub type EventSender = broadcast::Sender<Event>;
pub type EventReceiver = broadcast::Receiver<Event>;

#[derive(Clone)]
pub struct EventBus {
    senders: Arc<DashMap<String, EventSender>>,
    global_sender: EventSender,
}

impl EventBus {
    pub fn new() -> Self {
        let (global_sender, _) = broadcast::channel(1000);
        
        Self {
            senders: Arc::new(DashMap::new()),
            global_sender,
        }
    }

    pub fn subscribe(&self, event_type: Option<&str>) -> EventReceiver {
        match event_type {
            Some(event_type) => {
                let sender = self.senders
                    .entry(event_type.to_string())
                    .or_insert_with(|| {
                        let (sender, _) = broadcast::channel(100);
                        sender
                    })
                    .clone();
                sender.subscribe()
            }
            None => self.global_sender.subscribe(),
        }
    }

    pub fn publish(&self, event: Event) -> Result<(), anyhow::Error> {
        self.global_sender.send(event.clone()).ok();
        
        if let Some(sender) = self.senders.get(&event.event_type) {
            sender.send(event).ok();
        }
        
        Ok(())
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
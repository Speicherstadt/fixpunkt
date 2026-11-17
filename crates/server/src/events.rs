//! Live-Updates für die UI als Server-Sent Events. Vorbild: firsttris/haul crates/haul/src/events.rs

use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// Daten haben sich geändert; die UI lädt die betroffenen Listen neu.
    #[serde(rename_all = "camelCase")]
    Changed { topic: Topic, site_id: Option<i64> },
    /// Status eines Rechenauftrags
    #[serde(rename_all = "camelCase")]
    Job {
        id: i64,
        site_id: i64,
        status: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Topic {
    Sites,
    Captures,
    Regions,
    Jobs,
    Users,
    Markers,
    Plans,
}

#[derive(Clone)]
pub struct Events {
    tx: broadcast::Sender<Event>,
}

impl Default for Events {
    fn default() -> Self {
        Events {
            tx: broadcast::channel(256).0,
        }
    }
}

impl Events {
    pub fn send(&self, e: Event) {
        let _ = self.tx.send(e);
    }

    pub fn changed(&self, topic: Topic, site_id: Option<i64>) {
        self.send(Event::Changed { topic, site_id });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_for_the_ui() {
        let e = Event::Changed {
            topic: Topic::Captures,
            site_id: Some(3),
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"changed","topic":"captures","siteId":3}"#
        );
        let ev = Events::default();
        let mut rx = ev.subscribe();
        ev.send(Event::Job {
            id: 1,
            site_id: 2,
            status: "done".into(),
        });
        assert_eq!(
            rx.try_recv().unwrap(),
            Event::Job {
                id: 1,
                site_id: 2,
                status: "done".into()
            }
        );
    }
}

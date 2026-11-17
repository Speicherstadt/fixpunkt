//! Gemeinsamer Zustand des Servers.

use crate::config::Config;
use crate::db::Db;
use crate::events::Events;
use std::sync::Arc;
use tokio::sync::Notify;

pub struct App {
    pub db: Db,
    pub cfg: Config,
    pub events: Events,
    /// weckt die Job-Worker, sobald ein Auftrag dazukommt
    pub jobs_wakeup: Notify,
}

pub type AppRef = Arc<App>;

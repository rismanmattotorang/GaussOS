//! Event handlers

use crate::app::Action;
use tokio::sync::mpsc;

/// Event handler for async events
pub struct EventHandler {
    tick_rate: u64,
    action_tx: mpsc::UnboundedSender<Action>,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: u64, action_tx: mpsc::UnboundedSender<Action>) -> Self {
        Self { tick_rate, action_tx }
    }

    /// Start the event handler
    pub fn start(&self) {
        let tick_rate = self.tick_rate;
        let action_tx = self.action_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(tick_rate));

            loop {
                interval.tick().await;
                if action_tx.send(Action::Tick).is_err() {
                    break;
                }
            }
        });
    }
}

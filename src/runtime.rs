//! Runtime - Graceful shutdown and signal handling

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Shutdown signal broadcaster
#[derive(Clone)]
pub struct Shutdown {
    sender: broadcast::Sender<()>,
    triggered: Arc<RwLock<bool>>,
}

impl Default for Shutdown {
    fn default() -> Self { Self::new() }
}

impl Shutdown {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1);
        Self { sender, triggered: Arc::new(RwLock::new(false)) }
    }

    /// Subscribe to shutdown signal
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.sender.subscribe()
    }

    /// Trigger shutdown
    pub async fn trigger(&self) {
        let mut triggered = self.triggered.write().await;
        if !*triggered {
            *triggered = true;
            let _ = self.sender.send(());
        }
    }

    /// Check if shutdown was triggered
    pub async fn is_triggered(&self) -> bool {
        *self.triggered.read().await
    }
}

/// Install signal handlers and return shutdown handle
pub fn install_signal_handlers() -> Shutdown {
    let shutdown = Shutdown::new();
    let handle = shutdown.clone();

    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate()).expect("SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt()).expect("SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => tracing::info!("Received SIGTERM"),
                _ = sigint.recv() => tracing::info!("Received SIGINT"),
            }
        }

        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await.expect("Ctrl+C handler");
            tracing::info!("Received Ctrl+C");
        }

        handle.trigger().await;
    });

    shutdown
}

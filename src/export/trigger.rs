use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Decoupled way of triggering the exporter:
/// Any `mpsc::Sender` can be used as trigger source
pub struct ExportTrigger {
    trigger_receiver: mpsc::Receiver<()>,
}

impl ExportTrigger {
    /// Wait for the next trigger
    ///
    /// # Returns
    /// - `Some` whenever an export shall be triggered
    /// - `None` when the trigger stream is closed, so no exports shall be triggered anymore
    pub async fn next(&mut self) -> Option<()> {
        self.trigger_receiver.recv().await
    }

    /// Create a trigger whose `next()` function will immediately return `None`
    pub fn none() -> Self {
        let (_, trigger_receiver) = mpsc::channel(1);
        Self { trigger_receiver }
    }

    /// Create a trigger whose `next()` function will return `Some` repeatedly with the given period, and `None` after the cancellation token gets cancelled.
    pub fn periodic(period: Duration, cancellation_token: CancellationToken) -> Self {
        let (trigger_sender, trigger_receiver) = mpsc::channel(1);
        tokio::spawn(trigger_periodically(
            trigger_sender,
            period,
            cancellation_token,
        ));
        Self { trigger_receiver }
    }
}

async fn trigger_periodically(
    trigger_sender: mpsc::Sender<()>,
    period: Duration,
    cancellation_token: CancellationToken,
) {
    let mut interval = tokio::time::interval(period);
    log::info!(
        "Starting periodic export trigger with {} seconds",
        period.as_secs()
    );
    loop {
        tokio::select! {
            _ = interval.tick() => {
                log::debug!("Triggering now");
                let _ = trigger_sender.try_send(());
            }
            _ = cancellation_token.cancelled() => {
                log::info!("Cancellation requested, stopping periodic export trigger");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio_util::sync::CancellationToken;

    use crate::export::ExportTrigger;

    #[tokio::test]
    async fn trigger_none() {
        let mut trigger = ExportTrigger::none();
        assert!(trigger.next().await.is_none())
    }

    #[tokio::test]
    async fn trigger_periodic() {
        let cancellation_token = CancellationToken::new();
        let mut trigger =
            ExportTrigger::periodic(Duration::from_millis(1), cancellation_token.clone());
        assert!(trigger.next().await.is_some());
        assert!(trigger.next().await.is_some());
        cancellation_token.cancel();
        assert!(trigger.next().await.is_none());
    }
}

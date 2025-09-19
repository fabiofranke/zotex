use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    export::websocket::WebsocketTriggerBuilder,
    zotero_api::{api_key::ApiKey, client::UserId},
};

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

    /// Create a trigger based on websocket notifications from Zotero
    pub async fn websocket(
        api_key: ApiKey,
        user_id: UserId,
        cancellation_token: CancellationToken,
    ) -> anyhow::Result<Self> {
        let (trigger_sender, trigger_receiver) = mpsc::channel(1);
        let websocket_trigger = WebsocketTriggerBuilder::new(api_key, user_id, trigger_sender)
            .try_build()
            .await?;
        tokio::spawn(async move {
            if let Err(e) = websocket_trigger.run(cancellation_token).await {
                log::error!("WebSocket trigger encountered an error: {:?}", e);
            }
        });
        Ok(Self { trigger_receiver })
    }
}

#[cfg(test)]
mod tests {
    use crate::export::ExportTrigger;

    #[tokio::test]
    async fn trigger_none() {
        let mut trigger = ExportTrigger::none();
        assert!(trigger.next().await.is_none())
    }
}

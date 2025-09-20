use crate::zotero_api::{api_key::ApiKey, client::UserId};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::vec;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, connect_async,
    tungstenite::{self, Message},
};
use tokio_util::sync::CancellationToken;

type WebsocketStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct WebsocketTrigger {
    ws_stream: WebsocketStream,
    trigger_sender: mpsc::Sender<()>,
}

impl WebsocketTrigger {
    pub async fn run(mut self, cancel_token: CancellationToken) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    log::info!("WebSocket trigger cancelled");
                    return Ok(());
                }
                result = self.ws_stream.read_response() => {
                    match result {
                        Ok(Response::TopicUpdated { .. }) => {
                            log::info!("triggering export due to library change notification");
                            let _ = self.trigger_sender.try_send(());
                        },
                        Ok(other) => {
                            return Err(WebsocketError::UnexpectedResponse(other).into());
                        },
                        Err(e) => {
                            return Err(e.into());
                        }
                    }
                }
            }
        }
    }

    pub fn builder(
        api_key: ApiKey,
        user_id: UserId,
        trigger_sender: mpsc::Sender<()>,
    ) -> WebsocketTriggerBuilder {
        WebsocketTriggerBuilder {
            api_key,
            user_id,
            trigger_sender,
        }
    }
}

pub struct WebsocketTriggerBuilder {
    api_key: ApiKey,
    user_id: UserId,
    trigger_sender: mpsc::Sender<()>,
}

impl WebsocketTriggerBuilder {
    /// Try to build the WebSocket trigger, establishing the connection and subscribing to the user's library
    pub async fn try_build(self) -> anyhow::Result<WebsocketTrigger> {
        let mut ws_stream = self.connect().await?;
        self.subscribe(&mut ws_stream).await?;
        Ok(WebsocketTrigger {
            ws_stream,
            trigger_sender: self.trigger_sender,
        })
    }

    async fn connect(&self) -> Result<WebsocketStream, WebsocketError> {
        let (mut ws_stream, _) = connect_async("wss://stream.zotero.org").await?;
        let response = ws_stream.read_response().await?;
        if let Response::Connected { .. } = response {
            log::debug!("WebSocket connected");
            Ok(ws_stream)
        } else {
            log::error!("failed to connect to WebSocket");
            Err(WebsocketError::UnexpectedResponse(response))
        }
    }

    async fn subscribe(&self, ws_stream: &mut WebsocketStream) -> Result<(), WebsocketError> {
        let request = Request::CreateSubscriptions {
            subscriptions: vec![Subscription {
                api_key: self.api_key.0.clone(),
                topics: vec![format!("/users/{}", self.user_id)],
            }],
        };
        ws_stream.send_request(&request).await?;
        let response = ws_stream.read_response().await?;
        match response {
            Response::SubscriptionsCreated { errors, .. } if errors.is_empty() => {
                log::debug!("successfully subscribed to library updates");
                Ok(())
            }
            _ => {
                log::error!("failed to subscribe to library updates");
                Err(WebsocketError::UnexpectedResponse(response))
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum WebsocketError {
    #[error("connection error")]
    ConnectionError(#[from] tungstenite::Error),
    #[error("JSON error")]
    JsonError(#[from] serde_json::Error),
    #[error("unexpected response: {0:?}")]
    UnexpectedResponse(Response),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "action")]
enum Request {
    CreateSubscriptions { subscriptions: Vec<Subscription> },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Subscription {
    api_key: String,
    topics: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SubscriptionError {
    api_key: String,
    topic: String,
    error: String,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "event")]
enum Response {
    Connected {
        retry: u64,
    },
    SubscriptionsCreated {
        subscriptions: Vec<Subscription>,
        errors: Vec<SubscriptionError>,
    },
    TopicUpdated {
        topic: String,
        version: u64,
    },
}

trait WebsocketStreamExt {
    async fn read_response(&mut self) -> Result<Response, WebsocketError>;
    async fn send_request(&mut self, request: &Request) -> Result<(), WebsocketError>;
}

impl WebsocketStreamExt for WebsocketStream {
    async fn read_response(&mut self) -> Result<Response, WebsocketError> {
        loop {
            let msg = self
                .next()
                .await
                .ok_or(tungstenite::Error::ConnectionClosed)??;
            log::debug!("received message: {:?}", msg);
            match msg {
                Message::Text(bytes) => {
                    return serde_json::from_str::<Response>(bytes.as_str())
                        .inspect(|res| log::debug!("received response: {:?}", res))
                        .map_err(WebsocketError::from);
                }
                _ => log::debug!("ignoring non-text message: {:?}", msg),
            }
        }
    }

    async fn send_request(&mut self, request: &Request) -> Result<(), WebsocketError> {
        log::debug!("sending request: {:?}", request);
        let msg = serde_json::to_string(request)?;
        log::trace!("sending message: {:?}", msg);
        self.send(Message::Text(msg.into()))
            .await
            .map_err(WebsocketError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(Request::CreateSubscriptions { subscriptions: vec![Subscription { api_key: "abc".into(), topics: vec!["/users/123".into()] }] }, r#"{"action":"createSubscriptions","subscriptions":[{"apiKey":"abc","topics":["/users/123"]}]}"#)]
    fn test_serialize_request(#[case] input: Request, #[case] expected: &str) {
        let serialized = serde_json::to_string(&input).unwrap();
        assert_eq!(serialized, expected);
    }

    #[rstest]
    #[case(r#"{"event":"connected","retry":10}"#, Response::Connected { retry: 10 })]
    #[case(r#"{"event":"subscriptionsCreated","subscriptions":[{"apiKey":"xcv","topics":["/users/123"]}],"errors":[]}"#, Response::SubscriptionsCreated { subscriptions: vec![Subscription { api_key: "xcv".into(), topics: vec!["/users/123".into()] }], errors: vec![] })]
    #[case(r#"{"event":"subscriptionsCreated","subscriptions":[],"errors":[{"apiKey":"xcv","topic":"/users/123","error":"abc"}]}"#, Response::SubscriptionsCreated { subscriptions: vec![], errors: vec![SubscriptionError { api_key: "xcv".into(), topic: "/users/123".into(), error: "abc".into()}] })]
    #[case(r#"{"event":"topicUpdated","topic":"/users/123","version":105}"#, Response::TopicUpdated { topic: "/users/123".into(), version: 105 })]
    fn test_deserialize_response(#[case] input: &str, #[case] expected: Response) {
        let deserialized = serde_json::from_str::<Response>(input);
        assert_matches!(deserialized, Ok(response) => {
            assert_eq!(response, expected);
        });
    }
}

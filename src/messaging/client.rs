use crate::error::BotError;
use crate::messaging::models::{Command, Event};
use http::StatusCode;
use slack_morphism::{
    api::SlackApiChatPostMessageRequest,
    events::{SlackEventCallbackBody, SlackPushEventCallback},
    hyper_tokio::{SlackClientHyperConnector, SlackHyperClient},
    listener::{SlackClientEventsListenerEnvironment, SlackClientEventsUserState},
    SlackApiToken, SlackApiTokenValue, SlackChannelId, SlackClient, SlackClientSocketModeConfig,
    SlackClientSocketModeListener, SlackMessageContent, SlackSocketModeListenerCallbacks,
};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

pub struct AoCSlackClient {
    client: Arc<SlackHyperClient>,
}

pub struct MyEnvironment {
    pub sender: Arc<Sender<Event>>,
}

impl AoCSlackClient {
    pub fn new() -> Self {
        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()));
        Self { client }
    }

    pub async fn handle_messages_and_events(
        &self,
        tx: Sender<Event>,
        rx: Receiver<Event>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.listen_for_events(rx).await;
        self.start_slack_client_with_socket_mode(tx).await?;
        Ok(())
    }

    // Spaw listener for events and post corresponding annoucements/messages
    async fn listen_for_events(&self, mut rx: Receiver<Event>) {
        let client = self.client.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                // TODO: get slack channel name or id by config/settings
                // TODO: get other app env vars by config/settings

                let channel_id = SlackChannelId("C01T7GWLAVB".to_string());
                let app_token_value: SlackApiTokenValue =
                    config_env_var("SLACK_TEST_TOKEN").unwrap().into();
                let app_token: SlackApiToken = SlackApiToken::new(app_token_value);
                let session = client.open_session(&app_token);

                let response_text = event.to_string();

                let response = match event {
                    Event::CommandReceived(channel_id, thread_ts, _cmd) => {
                        SlackApiChatPostMessageRequest::new(
                            channel_id,
                            SlackMessageContent::new().with_text(response_text),
                        )
                        .with_thread_ts(thread_ts)
                    }
                    _ => SlackApiChatPostMessageRequest::new(
                        channel_id,
                        SlackMessageContent::new().with_text(response_text),
                    ),
                };

                if let Err(e) = session.chat_post_message(&response).await {
                    let error = BotError::Slack(e.to_string());
                    error!("{error}");
                };
            }
        });
    }

    async fn start_slack_client_with_socket_mode(
        &self,
        tx: Sender<Event>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
            .with_push_events(push_events_socket_mode_function);

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(self.client.clone())
                .with_error_handler(error_handler)
                .with_user_state(MyEnvironment {
                    sender: Arc::new(tx),
                }),
        );

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment.clone(),
            socket_mode_callbacks,
        );

        let app_token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_APP_TOKEN")?.into();
        let app_token: SlackApiToken = SlackApiToken::new(app_token_value);

        socket_mode_listener.listen_for(&app_token).await?;

        socket_mode_listener.serve().await;

        Ok(())
    }
}

async fn push_events_socket_mode_function(
    event: SlackPushEventCallback,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let SlackEventCallbackBody::Message(message) = event.event {
        match message.sender.bot_id {
            Some(_) => {
                // Message from bot, ignore
            }
            None => {
                // message from user, we will handle it if there is content and channel_id
                if let (Some(content), Some(channel_id)) = (message.content, message.origin.channel)
                {
                    if let Some(t) = content.text {
                        // TODO: Here we need to match commands by parsing t and if recognized command we sent
                        // an event so it can be processed.

                        let thread_ts = message.origin.ts; // to respond in thread

                        let states = states.read().await;
                        let state: Option<&MyEnvironment> =
                            states.get_user_state::<MyEnvironment>();
                        if let Some(env) = state {
                            let sender = env.sender.clone();

                            if let Err(e) = sender
                                .send(Event::CommandReceived(channel_id, thread_ts, Command::Help))
                                .await
                            {
                                error!("{}", e);
                            };
                        };
                    };
                };
            }
        }
    };
    Ok(())
}

fn error_handler(
    err: Box<dyn std::error::Error + Send + Sync>,
    _client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> StatusCode {
    let error = BotError::Slack(err.to_string());
    error!("{error}");

    // This return value should be OK if we want to return successful ack to the Slack server using Web-sockets
    // https://api.slack.com/apis/connections/socket-implement#acknowledge
    // so that Slack knows whether to retry
    StatusCode::OK
}

pub async fn client_with_socket_mode(
    client: Arc<SlackHyperClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let socket_mode_callbacks =
        SlackSocketModeListenerCallbacks::new().with_push_events(push_events_socket_mode_function);

    let listener_environment = Arc::new(
        SlackClientEventsListenerEnvironment::new(client.clone()).with_error_handler(error_handler),
    );

    let socket_mode_listener = SlackClientSocketModeListener::new(
        &SlackClientSocketModeConfig::new(),
        listener_environment.clone(),
        socket_mode_callbacks,
    );

    let app_token_value: SlackApiTokenValue = config_env_var("SLACK_TEST_APP_TOKEN")?.into();
    let app_token: SlackApiToken = SlackApiToken::new(app_token_value);

    socket_mode_listener.listen_for(&app_token).await?;

    socket_mode_listener.serve().await;

    Ok(())
}

pub fn config_env_var(name: &str) -> Result<String, String> {
    std::env::var(name).map_err(|e| format!("{}: {}", name, e))
}

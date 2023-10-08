use crate::error::{BotError, BotResult};
use crate::messaging::models::MyEvent;
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
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::{error, info};

async fn push_events_socket_mode_function(
    event: SlackPushEventCallback,
    client: Arc<SlackHyperClient>,
    _states: SlackClientEventsUserState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Only watch Message events for now. To be switched to match cases if we want other behaviors
    // on other event types.
    if let SlackEventCallbackBody::Message(message) = event.event {
        match message.sender.bot_id {
            Some(_) => {
                // Abort if message from bot
                return Ok(());
            }
            None => {
                // message from user, alright, we respond
                // let channel = message.origin.channel.unwrap();
                if let (Some(content), Some(channel_id)) = (message.content, message.origin.channel)
                {
                    if let Some(t) = content.text {
                        // TODO: send a message to the queue wiht channel ID, ts, and
                        // command. So the post messages (internal and external) are all
                        // handled by same service. So we need the sender here.
                        // Same example than on the issue sync ex

                        info!("Received message in channel id {channel_id}, checking if command");
                        let ts = message.origin.ts; // to respond in thread
                        let app_token_value: SlackApiTokenValue =
                            config_env_var("SLACK_TEST_TOKEN")?.into();
                        let app_token: SlackApiToken = SlackApiToken::new(app_token_value);
                        let session = client.open_session(&app_token);
                        let response_text = format!(":repeat: Received your query '{t}'");

                        let response = SlackApiChatPostMessageRequest::new(
                            channel_id,
                            SlackMessageContent::new().with_text(response_text),
                        )
                        .with_thread_ts(ts);
                        session.chat_post_message(&response).await?;
                    };
                }
            }
        }
    }
    Ok(())
}

fn test_error_handler(
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
        SlackClientEventsListenerEnvironment::new(client.clone())
            .with_error_handler(test_error_handler),
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

pub async fn initialize_messaging(
    mut rx: UnboundedReceiver<MyEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()));
    let client_clone = client.clone();

    // Bot announcements.
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            // TODO: get slack channel name or id by config/settings
            // TODO: get other app env vars by config/settings
            let channel_id = SlackChannelId("C01T7GWLAVB".to_string());
            let app_token_value: SlackApiTokenValue =
                config_env_var("SLACK_TEST_TOKEN").unwrap().into();
            let app_token: SlackApiToken = SlackApiToken::new(app_token_value);
            let session = client_clone.open_session(&app_token);
            let response_text = format!(":tada: {}", message.event);

            let response = SlackApiChatPostMessageRequest::new(
                channel_id,
                SlackMessageContent::new().with_text(response_text),
            );
            // let _s = session.chat_post_message(&response).await.map_err(|_| {});
            if let Err(e) = session.chat_post_message(&response).await {
                let error = BotError::Slack(e.to_string());
                error!("{error}");
            };
        }
    });

    // Handle messages from users
    client_with_socket_mode(client.clone()).await?;

    Ok(())
}


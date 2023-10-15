use crate::config;
use crate::error::BotError;
use crate::messaging::events::{Command, Event};
use crate::storage::MemoryCache;
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
use tracing::error;

struct MyEnvironment {
    sender: Arc<Sender<Event>>,
    cache: MemoryCache,
}

pub struct AoCSlackClient {
    client: Arc<SlackHyperClient>,
}

impl AoCSlackClient {
    pub fn new() -> Self {
        let client = Arc::new(SlackClient::new(SlackClientHyperConnector::new()));
        Self { client }
    }

    pub async fn handle_messages_and_events(
        &self,
        cache: MemoryCache,
        tx: Sender<Event>,
        rx: Receiver<Event>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.listen_for_events(rx).await;
        self.start_slack_client_with_socket_mode(cache.clone(), tx)
            .await?;
        Ok(())
    }

    // Spaw listener for events and post corresponding annoucements/messages
    async fn listen_for_events(&self, mut rx: Receiver<Event>) {
        let client = self.client.clone();

        tokio::spawn(async move {
            let settings = &config::SETTINGS;
            while let Some(event) = rx.recv().await {
                let channel_id = SlackChannelId(settings.slack_default_channel.to_string());
                let app_token_value: SlackApiTokenValue = settings.slack_token.to_string().into();
                let app_token: SlackApiToken = SlackApiToken::new(app_token_value);
                let session = client.open_session(&app_token);

                let response_text = event.to_string();

                let response = match &event {
                    Event::CommandReceived(channel_id, thread_ts, _cmd) => {
                        // let data = cache.data.lock().unwrap();
                        // // TODO: inject timestamp too
                        // let ranking = data.leaderboard.standings_by_local_score();

                        SlackApiChatPostMessageRequest::new(
                            channel_id.clone(),
                            SlackMessageContent::new().with_text(response_text),
                        )
                        .with_thread_ts(thread_ts.clone())
                    }
                    _ => SlackApiChatPostMessageRequest::new(
                        channel_id.clone(),
                        SlackMessageContent::new().with_text(response_text),
                    ),
                };

                match session.chat_post_message(&response).await {
                    Err(e) => {
                        let error = BotError::Slack(e.to_string());
                        error!("{error}");
                    }
                    Ok(res) => {
                        // If Solution thread initialization, post a first message in thread
                        if let Event::DailySolutionsThreadToInitialize(_day) = event {
                            let thread_ts = res.ts;
                            let message = format!("Show me your best move!");
                            let first_thread_message = SlackApiChatPostMessageRequest::new(
                                channel_id,
                                SlackMessageContent::new().with_text(message),
                            )
                            .with_thread_ts(thread_ts);
                            if let Err(e) = session.chat_post_message(&first_thread_message).await {
                                let error = BotError::Slack(e.to_string());
                                error!("{error}");
                            };
                        }
                    }
                }
            }
        });
    }

    async fn start_slack_client_with_socket_mode(
        &self,
        cache: MemoryCache,
        tx: Sender<Event>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let settings = &config::SETTINGS;
        let socket_mode_callbacks = SlackSocketModeListenerCallbacks::new()
            .with_push_events(push_events_socket_mode_function);

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(self.client.clone())
                .with_error_handler(error_handler)
                .with_user_state(MyEnvironment {
                    sender: Arc::new(tx),
                    cache,
                }),
        );

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment.clone(),
            socket_mode_callbacks,
        );

        let app_token_value: SlackApiTokenValue = settings.slack_app_token.to_string().into();
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
        // Only respond to messages from users (no bot_id)
        if let None = message.sender.bot_id {
            // message from user, we will handle it if there is content and channel_id
            if let (Some(content), Some(channel_id)) = (message.content, message.origin.channel) {
                if let Some(t) = content.text {
                    if Command::is_command(&t) {
                        let states = states.read().await;
                        let state: Option<&MyEnvironment> =
                            states.get_user_state::<MyEnvironment>();
                        if let Some(env) = state {
                            let cache = env.cache.clone();
                            let sender = env.sender.clone();

                            let cmd = {
                                let data = cache.data.lock().unwrap();
                                Command::build_from(t, &data)
                            };

                            let thread_ts = message.origin.ts; // to respond in thread

                            if let Err(e) = sender
                                .send(Event::CommandReceived(channel_id, thread_ts, cmd))
                                .await
                            {
                                error!("{}", e);
                            };
                            // }
                        };
                    };
                };
            };
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

// fn has_command(message: &Option<String>) -> Option<String> {
//     match *message {
//         Some(ref text) => {
//             let re =
//                 Regex::new(r"!help\s(?P<command>.*?)$").expect("command regex should not fail");
//             let command_result = re
//                 .captures(text)
//                 .map(|capture| String::from(&capture["command"]));
//             if command_result == Some("".to_owned()) {
//                 None
//             } else {
//                 command_result
//             }
//         }
//         _ => None,
//     }
// }

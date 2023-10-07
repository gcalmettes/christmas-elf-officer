use reqwest::Client;

use futures::stream::StreamExt;
use futures::stream::TryStreamExt; // for .map_err()
use tokio::io::AsyncBufReadExt; // for .lines()
use tokio_util::io::StreamReader; // for .next()

use crate::error::{convert_err, BotResult};
use crate::messaging::models::{Command, Msg};

pub struct MatterBridgeClient {
    http_client: Client,
    base_url: String,
}

impl MatterBridgeClient {
    pub fn new(base_url: String) -> Self {
        let http_client = Client::builder().build().unwrap();
        Self {
            http_client,
            base_url,
        }
    }

    pub async fn read_stream(&self) -> BotResult<()> {
        let url = format!("{}{}", self.base_url, "/api/stream");

        let mut stream = self.http_client.get(&url).send().await?.bytes_stream();

        while let Some(item) = stream.next().await {
            // b"{\"text\":\"Hello xylophone\",\"channel\":\"api\",\"username\":\"gcalmettes\",\"userid\":\"\",\"avatar\":\"\",\"account\":\"api.myapi\",\"event\":\"\",\"protocol\":\"api\",\"gateway\":\"mygateway\",\"parent_id\":\"\",\"timestamp\":\"2023-09-16T13:45:55.303052409Z\",\"id\":\"\",\"Extra\":null}\n"

            // let parsed_msg = serde_json::from_slice::<Msg>(&item.unwrap()).unwrap();
            match item {
                Ok(msg) => match serde_json::from_slice::<Msg>(&msg) {
                    Ok(msg) => match msg.as_command() {
                        Ok(Command::Help) => {
                            println!("Help command: {:?}", msg);
                        }
                        Ok(Command::Fast(_day)) => {
                            println!("Fast command: {:?}", msg);
                        }
                        _ => {
                            println!("NOT A COMMAND: {:?}", msg);
                            ()
                        }
                    },
                    Err(e) => {
                        println!("ERROR: {:?}", e);
                    }
                },
                Err(e) => {
                    println!("ERROR: {:?}", e);
                }
            };
        }

        Ok(())
    }

    // pub async fn read_stream(&self) -> BotResult<()> {
    //     let url = format!("{}{}", self.base_url, "/api/stream");

    //     let stream = self.http_client.get(&url).send().await?.bytes_stream();

    //     let reader = StreamReader::new(stream.map_err(convert_err));
    //     let mut lines = reader.lines();
    //     while let Some(line) = lines.next_line().await? {
    //         match serde_json::from_str::<Msg>(&line) {
    //             Ok(msg) => match msg.as_command() {
    //                 Ok(Command::Help) => {
    //                     println!("Help command: {:?}", msg);
    //                 }
    //                 Ok(Command::Fast(_day)) => {
    //                     println!("Fast command: {:?}", msg);
    //                 }
    //                 _ => {
    //                     println!("NOT A COMMAND: {:?}", msg);
    //                     ()
    //                 }
    //             },
    //             Err(e) => {
    //                 println!("ERROR: {:?}", e);
    //             }
    //         }
    //     }

    //     Ok(())
    // }
}

use reqwest::Client;

// use futures::stream::{Stream, StreamExt};

use futures::stream::TryStreamExt; // for .map_err()
use tokio::io::AsyncBufReadExt; // for .lines()
use tokio_util::io::StreamReader;

use crate::error::{convert_err, BotResult};
use crate::messaging::models::Msg;

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

    // async fn acquire_stream(
    //     &self,
    // ) -> BotResult<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>> {
    //     let url = format!("{}{}", self.base_url, "/api/stream");

    //     let stream = self.http_client.get(&url).send().await?.bytes_stream();

    //     Ok(stream)
    // }

    // pub async fn read_stream(&self) -> BotResult<()> {
    //     let mut stream = self.acquire_stream().await?;

    //     while let Some(item) = stream.next().await {
    //         // b"{\"text\":\"Hello xylophone\",\"channel\":\"api\",\"username\":\"gcalmettes\",\"userid\":\"\",\"avatar\":\"\",\"account\":\"api.myapi\",\"event\":\"\",\"protocol\":\"api\",\"gateway\":\"mygateway\",\"parent_id\":\"\",\"timestamp\":\"2023-09-16T13:45:55.303052409Z\",\"id\":\"\",\"Extra\":null}\n"

    //         // let parsed_msg = serde_json::from_slice::<Msg>(&item.unwrap()).unwrap();
    //         match item {
    //             Ok(i) => {
    //                 match serde_json::from_slice::<Msg>(&i) {
    //                     Ok(msg) => {
    //                         println!("MSG: {:?}", msg);
    //                     }
    //                     Err(e) => {
    //                         println!("ERROR: {:?}", e);
    //                     }
    //                 };
    //             }
    //             Err(e) => {
    //                 println!("ERROR: {:?}", e);
    //             }
    //         };
    //     }

    //     Ok(())
    // }

    // async fn acquire_stream(&self) -> BotResult<tokio::io::Lines<StreamReader<futures::stream::MapErr<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>, fn(reqwest::Error) -> std::io::Error {convert_error}>, bytes::Bytes>>> {
    // async fn read_stream(
    //     &self,
    // ) -> BotResult<
    //     tokio::io::Lines<
    //         StreamReader<
    //             futures::stream::MapErr<
    //                 impl Stream<Item = Result<bytes::Bytes, reqwest::Error>>,
    //                 std::io::Error,
    //             >,
    //             bytes::Bytes,
    //         >,
    //     >,
    // > {
    pub async fn read_stream(&self) -> BotResult<()> {
        let url = format!("{}{}", self.base_url, "/api/stream");

        let stream = self.http_client.get(&url).send().await?.bytes_stream();

        let reader = StreamReader::new(stream.map_err(convert_err));
        let mut lines = reader.lines();
        while let Some(line) = lines.next_line().await? {
            match serde_json::from_str::<Msg>(&line) {
                Ok(msg) => {
                    println!("MSG: {:?}", msg);
                }
                Err(e) => {
                    println!("ERROR: {:?}", e);
                }
            }
        }

        Ok(())
    }
}

use serde::Deserialize;

//  {
//   "text": "world",
//   "channel": "bot-test",
//   "username": "Guillaume 5️⃣",
//   "userid": "UQMNRPW0M",
//   "avatar": "https://avatars.slack-edge.com/2019-11-25/836953095059_25e65716133f521ebef2_48.jpg",
//   "account": "slack.test",
//   "event": "",
//   "protocol": "slack",
//   "gateway": "mygateway",
//   "parent_id": "slack 1694977117.342199",
//   "timestamp": "2023-09-17T18:58:41.71696628Z",
//   "id": "",
//   "Extra": {}
//  }
// ]

#[derive(Debug, Clone, Deserialize)]
pub struct Msg {
    text: String,
    channel: String,
    username: String,
    userid: String,
    avatar: String,
    account: String,
    event: String,
    protocol: String,
    gateway: String,
    parent_id: String,
    timestamp: String,
    id: String,
    // extra: Option<HashMap<String, String>>,
}

use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug)]
pub enum Command {
    Help,
    Fast(Option<u8>),
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Command, ()> {
        if s.starts_with("!help") {
            Ok(Command::Help)
        } else if s.starts_with("!fast") {
            match s.split_once(" ") {
                Some((_, i)) => match i.parse::<u8>() {
                    Ok(day) => Ok(Command::Fast(Some(day))),
                    Err(e) => {
                        println!("EEERRRR {e}");
                        Err(())
                    }
                },
                None => {
                    println!("YYYYY");
                    Ok(Command::Fast(None))
                }
            }
        } else {
            println!("NOPE {s}");
            Err(())
        }
    }

    // match s {
    //     _ if s.starts_with("!help") => Ok(Command::Help),
    //     _ if s.starts_with("!fast") => {
    //         println!("GOOOGGG {}", s);
    //         match s.split_once(" ") {
    //             Some((_, i)) => match i.parse::<u8>() {
    //                 Ok(day) => Ok(Command::Fast(Some(day))),
    //                 Err(e) => {
    //                     println!("EEERRRR {e}");
    //                     Err(())
    //                 }
    //             },
    //             None => {
    //                 println!("YYYYY");
    //                 Ok(Command::Fast(None))
    //             }
    //         }
    //     }
    //     _ => Err(()),
    // }
    // }
}

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
    pub text: String,
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

impl Msg {
    pub fn as_command(&self) -> Result<Command, ()> {
        self.text.parse::<Command>()
    }
}

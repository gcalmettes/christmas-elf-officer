use ceo_bot::aoc::AoC;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let aoc_client = AoC::new(
        "http://localhost:5001".to_string(),
        Duration::new(5, 0),
        261166,
        "yolo".to_string(),
    );

    // let resp = aoc_client.global_leaderboard(2022, 1).await;
    let resp = aoc_client.private_leaderboard(2022).await;

    match resp {
        Ok(response) => println!("{:?}", response),
        Err(e) => println!("{}", e),
    }

    Ok(())
}

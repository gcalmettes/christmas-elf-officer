use ceo_bot::client::aoc::AoC;
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
    // let resp = aoc_client.private_leaderboard(2022).await;
    let resp = aoc_client.private_leaderboard(2000).await;

    match resp {
        Ok(leaderboard) => {
            // println!("{:?}", leaderboard.standings_by_local_score())
            println!("{:?}", leaderboard.standings_by_number_of_stars());
            println!("{:?}", leaderboard.standings_by_global_score());
            (1..=25).for_each(|d| {
                println!("\n>> DAY {}", d);
                leaderboard
                    .standings_for_day(d)
                    .iter()
                    .for_each(|(name, score)| {
                        println!("  {} {}", name, score);
                    })
            })
        }
        Err(e) => println!("{}", e),
    }

    Ok(())
}

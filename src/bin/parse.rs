use rocketlaunch_bot::types::Launches;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = reqwest::get("https://fdo.rocketlaunch.live/json/launches/next/5")
        .await?
        .text()
        .await?;
    // println!("{}", text);
    let launches: Launches = serde_json::from_str(&text)?;
    println!("{:#?}", launches);

    Ok(())
}

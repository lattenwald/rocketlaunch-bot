use teloxide::{requests::Requester, types::ChatId, utils::markdown};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::{
    bot::MyBot,
    db::Db,
    types::{Launch, Launches, RLError},
};

const API_URL: &str = "https://fdo.rocketlaunch.live/json/launches/next/5";

#[tracing::instrument]
pub async fn fetch() -> Result<Vec<Launch>, RLError> {
    info!("fetching");
    let text = reqwest::get(API_URL).await?.text().await?;
    let launches: Launches = serde_json::from_str(&text)?;
    Ok(launches.launches)
}

#[tracing::instrument(skip_all)]
pub async fn worker(db: Db, bot: MyBot, cancellation: CancellationToken) {
    loop {
        match worker_loop(&db, &bot, &cancellation).await {
            Ok(()) => {
                return;
            }
            Err(err) => {
                error!("worker_loop fail: {}", err);
                tokio::select! {
                    _ = cancellation.cancelled() => {
                        return;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {}
                }
            }
        }
    }
}

async fn worker_loop(
    db: &Db,
    bot: &MyBot,
    cancellation: &CancellationToken,
) -> Result<(), RLError> {
    loop {
        let launches = fetch().await?;
        for launch in launches {
            for chat_id in db.get_unnotified(launch.id)? {
                bot.send_message(ChatId(chat_id), markdown::escape(&launch.quicktext))
                    .await?;
                db.set_notified(chat_id, launch.id)?;
            }
        }

        tokio::select! {
            _ = cancellation.cancelled() => {
                return Ok(());
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(60)) => {}
        }
    }
}

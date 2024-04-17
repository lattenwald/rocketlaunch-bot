use std::fmt::Write;

use chrono::{DurationRound, TimeDelta, Utc};
use humantime::format_duration;
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
        let now = Utc::now()
            .duration_round(TimeDelta::try_minutes(1).unwrap())
            .unwrap();
        for launch in launches {
            let Some(t0) = launch.t0 else {
                continue;
            };
            let mut text = format!(
                "[{} \\- {}](https://rocketlaunch.live/launch/{})\n{} \\(in *{}*\\)\n{}",
                markdown::escape(&launch.provider.name),
                markdown::escape(&launch.vehicle.name),
                markdown::escape(&launch.slug),
                markdown::escape(&format!("{}", t0)),
                markdown::escape(&format!(
                    "{}",
                    format_duration((t0 - now).to_std().unwrap())
                )),
                markdown::escape(&format!("{}", launch.pad)),
            );

            if let Some(desc) = launch.mission_description {
                let _ = write!(text, "\n\n{}", markdown::escape(&desc));
            }

            if launch.suborbital {
                let _ = write!(text, "\n\nsuborbital");
            }
            for chat_id in db.get_unnotified(launch.id, t0)? {
                info!("notifying {} about launch {}", chat_id, launch.id);
                bot.send_message(ChatId(chat_id), &text).await?;
                db.set_notified(chat_id, launch.id, t0)?;
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

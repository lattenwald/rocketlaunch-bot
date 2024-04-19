use chrono::{Duration, DurationRound, TimeDelta, Utc};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::{
    bot::{launches_notify, MyBot},
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

#[tracing::instrument(skip_all)]
async fn worker_loop(
    db: &Db,
    bot: &MyBot,
    cancellation: &CancellationToken,
) -> Result<(), RLError> {
    loop {
        let launches = fetch().await?;
        db.set_launches(&launches)?;
        launches_notify(bot, db, &launches).await?;

        let next_run_in: Duration = {
            let next_min = (Utc::now() + Duration::try_minutes(1).unwrap())
                .duration_round(TimeDelta::try_minutes(1).unwrap())
                .unwrap();
            next_min - Utc::now()
        };

        debug!("next run in {}", &next_run_in);

        tokio::select! {
            _ = cancellation.cancelled() => {
                return Ok(());
            }
            _ = tokio::time::sleep(next_run_in.to_std().unwrap()) => {}
        }
    }
}

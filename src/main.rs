use clap::Parser;
use rocketlaunch_bot::{bot::init_bot, config::Args, db::Db, fetch::worker};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[tokio::main]
#[tracing::instrument]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)
        .expect("failed setting up tracing subscriber");

    let config = Args::parse().get_config();
    dbg!(&config);

    let (cancellation, force_stop) = spawn_shutdown();

    let db = Db::open("db").expect("failed opening db");

    let (bot, mut bot_dispatcher) = init_bot(config.bot, db.clone()).await;

    let worker = tokio::spawn(worker(db, bot, cancellation.clone()));
    let dispatcher_handle = tokio::spawn(async move {
        tokio::select! {
            _ = bot_dispatcher.dispatch() => (),
            _ = cancellation.cancelled() => (),
        };
    });

    let w = async {
        if let Err(err) = dispatcher_handle.await {
            warn!("tg bot fail: {}", err);
        } else {
            info!("tg bot complete");
        }

        if let Err(err) = worker.await {
            warn!("worker fail: {}", err);
        } else {
            info!("worker complete");
        }
    };

    {
        let c = force_stop.cancelled();
        futures::pin_mut!(w);
        futures::pin_mut!(c);
        futures::future::select(w, c).await;
    }
}

fn spawn_shutdown() -> (CancellationToken, CancellationToken) {
    let cancellation = CancellationToken::new();
    let force_stop = CancellationToken::new();
    {
        let cancellation = cancellation.clone();
        let force_stop = force_stop.clone();
        tokio::spawn(async move {
            loop {
                tokio::signal::ctrl_c().await.unwrap();
                info!("Ctrl-C signal received, wait for started tasks");
                cancellation.cancel();

                tokio::signal::ctrl_c().await.unwrap();
                warn!("Chill dude, have some patience...");

                tokio::signal::ctrl_c().await.unwrap();
                warn!("Aight aight, I gotcha, FORCE STOP");
                force_stop.cancel();
            }
        });
    }
    (cancellation, force_stop)
}

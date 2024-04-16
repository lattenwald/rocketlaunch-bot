use clap::Parser;
use rocketlaunch_bot::{bot::init_bot, config::Args};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let config = Args::parse().get_config();
    dbg!(&config);

    let (cancellation, force_stop) = spawn_shutdown();

    let bot = tokio::spawn(init_bot(config.bot, cancellation.clone()));

    let w = async {
        if let Err(err) = bot.await {
            eprintln!("tg bot fail: {}", err);
        } else {
            println!("tg bot complete");
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
        // Просто клонируем токен, не создавая дочерний
        let cancellation = cancellation.clone();
        let force_stop = force_stop.clone();
        tokio::spawn(async move {
            loop {
                tokio::signal::ctrl_c().await.unwrap();

                // Ставим пометку о завершении, новые задачи уже не будем брать
                eprintln!("Ctrl-C signal received, wait for started tasks");
                cancellation.cancel();

                // На третий ctrl+c force stop
                tokio::signal::ctrl_c().await.unwrap();
                eprintln!("Chill dude, have some patience...");
                tokio::signal::ctrl_c().await.unwrap();
                eprintln!("Aight aight, I gotcha, FORCE STOP");
                force_stop.cancel();
            }
        });
    }
    (cancellation, force_stop)
}

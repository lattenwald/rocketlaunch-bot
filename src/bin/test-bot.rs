use clap::Parser;
use rocketlaunch_bot::{bot::init_bot, config::BotConfig, db::Db};

#[derive(Debug, Clone, Parser)]
struct Args {
    /// bot token
    #[clap(flatten)]
    bot: BotConfig,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    dbg!(&args);

    let db = Db::open("test_db").unwrap();

    let (_bot, mut dispatcher) = init_bot(args.bot, db).await;
    dispatcher.dispatch().await;
}

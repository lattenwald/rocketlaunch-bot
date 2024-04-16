use clap::Parser;
use rocketlaunch_bot::{bot::init_bot, config::BotConfig};

#[derive(Debug, Clone, Parser)]
struct Args {
    /// bot token
    #[clap(flatten)]
    bot: BotConfig,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    init_bot(args.bot).await;
}

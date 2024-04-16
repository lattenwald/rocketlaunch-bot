use clap::Parser;
use rocketlaunch_bot::{bot::init_bot, config::BotConfig};
use tokio_util::sync::CancellationToken;

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

    let cancellation = CancellationToken::new();

    init_bot(args.bot, cancellation).await;
}

use teloxide::{
    adaptors::{trace, CacheMe, DefaultParseMode, Throttle, Trace},
    dispatching::{Dispatcher, UpdateFilterExt},
    prelude::*,
    requests::ResponseResult,
    types::{Message, ParseMode, Update},
    utils::command::BotCommands,
    Bot,
};
use tokio_util::sync::CancellationToken;

use crate::config::BotConfig;

type MyBot = Trace<Throttle<CacheMe<DefaultParseMode<Bot>>>>;

pub async fn init_bot(config: BotConfig, cancellation: CancellationToken) {
    let bot: MyBot = Bot::new(config.token.clone())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .throttle(Default::default())
        .trace(trace::Settings::TRACE_EVERYTHING);

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<UnauthorizedCommand>()
                .endpoint(unauthorized_command_handler),
        )
        .branch(
            dptree::filter(|cfg: BotConfig, msg: Message| cfg.admin_chats.contains(&msg.chat.id.0))
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(command_handler),
                ),
        );

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![config])
        .build();

    tokio::select! {
        _ = dispatcher.dispatch() => (),
        _ = cancellation.cancelled() => (),
    };
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "Доступные команды:")]
enum UnauthorizedCommand {
    #[command(description = "id текущего чата")]
    Id,
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "Доступные команды:")]
enum Command {
    #[command(description = "помощь")]
    Help,

    #[command(description = "id текущего чата")]
    Id,
}

async fn unauthorized_command_handler(
    bot: MyBot,
    msg: Message,
    cmd: UnauthorizedCommand,
) -> ResponseResult<()> {
    match cmd {
        UnauthorizedCommand::Id => {
            bot.send_message(msg.chat.id, format!("`{}`", msg.chat.id))
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

async fn command_handler(bot: MyBot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .reply_to_message_id(msg.id)
                .await?;
        }
        Command::Id => {
            bot.send_message(msg.chat.id, format!("`{}`", msg.chat.id))
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

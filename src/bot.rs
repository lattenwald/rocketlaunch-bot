use teloxide::{
    adaptors::{trace, CacheMe, Throttle, Trace},
    dispatching::{Dispatcher, UpdateFilterExt},
    prelude::*,
    requests::ResponseResult,
    types::{Message, ParseMode, Update},
    utils::command::BotCommands,
    Bot,
};

use crate::config::BotConfig;

type MyBot = Trace<Throttle<CacheMe<Bot>>>;

pub async fn init_bot(config: BotConfig) {
    let bot: MyBot = Bot::new(config.token)
        .cache_me()
        .throttle(Default::default())
        .trace(trace::Settings::TRACE_EVERYTHING);

    let handler = Update::filter_message().branch(
        dptree::entry()
            .filter_command::<UnauthorizedCommand>()
            .endpoint(unauthorized_command_handler),
    );

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler).build();

    dispatcher.dispatch().await;
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "Доступные команды:")]
enum UnauthorizedCommand {
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
                .parse_mode(ParseMode::MarkdownV2)
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

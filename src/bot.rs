use teloxide::{
    adaptors::{trace, CacheMe, DefaultParseMode, Throttle, Trace},
    dispatching::{Dispatcher, UpdateFilterExt},
    prelude::*,
    requests::ResponseResult,
    types::{Message, ParseMode, Update},
    utils::{command::BotCommands, markdown},
    Bot,
};
use tracing::info;

use crate::{config::BotConfig, db::Db};

pub type MyBot = Trace<Throttle<CacheMe<DefaultParseMode<Bot>>>>;
pub type MyDispatcher =
    Dispatcher<MyBot, teloxide::RequestError, teloxide::dispatching::DefaultKey>;

pub async fn init_bot(config: BotConfig, db: Db) -> (MyBot, MyDispatcher) {
    let bot: MyBot = Bot::new(config.token.clone())
        .parse_mode(ParseMode::MarkdownV2)
        .cache_me()
        .throttle(Default::default())
        .trace(trace::Settings::TRACE_EVERYTHING);

    let handler = Update::filter_message()
        .branch(
            dptree::filter(|cfg: BotConfig, msg: Message| cfg.admin_chats.contains(&msg.chat.id.0))
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .endpoint(command_handler),
                ),
        )
        .branch(
            dptree::entry()
                .filter_command::<UnauthorizedCommand>()
                .endpoint(unauthorized_command_handler),
        );

    (
        bot.clone(),
        Dispatcher::builder(bot.clone(), handler)
            .dependencies(dptree::deps![config, db])
            .build(),
    )
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "*Команды:*")]
enum UnauthorizedCommand {
    #[command(description = "id текущего чата")]
    Id,

    #[command(description = "подписаться на запуски")]
    Subscribe,

    #[command(description = "отписаться от запусков")]
    Unsubscribe,

    #[command(description = "помощь")]
    Help,
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "*Админские команды:*")]
enum Command {
    #[command(description = "помощь")]
    Help,
}

#[tracing::instrument(skip_all)]
async fn unauthorized_command_handler(
    bot: MyBot,
    msg: Message,
    cmd: UnauthorizedCommand,
    db: Db,
) -> ResponseResult<()> {
    info!("handling: {:?}", cmd);
    match cmd {
        UnauthorizedCommand::Id => {
            bot.send_message(msg.chat.id, format!("`{}`", msg.chat.id))
                .reply_to_message_id(msg.id)
                .await?;
        }
        UnauthorizedCommand::Help => {
            bot.send_message(msg.chat.id, UnauthorizedCommand::descriptions().to_string())
                .reply_to_message_id(msg.id)
                .await?;
        }
        UnauthorizedCommand::Subscribe => match db.subscribe(msg.chat.id.0) {
            Ok(_) => {
                bot.send_message(
                    msg.chat.id,
                    markdown::escape("Есть подписка, ждите уведомлений!"),
                )
                .reply_to_message_id(msg.id)
                .await?;
            }
            Err(err) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Ошибка подписки:\n```\n{}\n```",
                        markdown::escape(&format!("{:?}", err))
                    ),
                )
                .reply_to_message_id(msg.id)
                .await?;
            }
        },
        UnauthorizedCommand::Unsubscribe => match db.unsubscribe(msg.chat.id.0) {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Отписались")
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            Err(err) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Ошибка отписки:\n```\n{}\n```",
                        markdown::escape(&format!("{:?}", err))
                    ),
                )
                .reply_to_message_id(msg.id)
                .await?;
            }
        },
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn command_handler(bot: MyBot, msg: Message, cmd: Command) -> ResponseResult<()> {
    info!("handling: {:?}", cmd);
    match cmd {
        Command::Help => {
            let commands = [
                Command::descriptions().to_string(),
                UnauthorizedCommand::descriptions().to_string(),
            ];
            bot.send_message(msg.chat.id, commands.join("\n\n"))
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

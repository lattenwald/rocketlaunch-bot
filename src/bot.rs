use std::fmt::Write;

use chrono::{Duration, DurationRound, TimeDelta, Utc};
use humantime::format_duration;
use teloxide::{
    adaptors::{trace, CacheMe, DefaultParseMode, Throttle, Trace},
    dispatching::{Dispatcher, UpdateFilterExt},
    prelude::*,
    requests::ResponseResult,
    types::{Message, MessageId, ParseMode, Update},
    utils::{command::BotCommands, markdown},
    ApiError, Bot, RequestError,
};
use tracing::{info, warn};

use crate::{
    config::BotConfig,
    db::Db,
    types::{Launch, RLError},
};

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
                        .filter_command::<AdminCommand>()
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
#[command(rename_rule = "snake_case", description = "*Commands:*")]
enum UnauthorizedCommand {
    #[command(description = "current chat id")]
    Id,

    #[command(description = "subscribe to launches notifications")]
    Start,

    #[command(description = "unsubscribe from launches notifications")]
    Stop,

    #[command(description = "show launches")]
    Launches,

    #[command(description = "show next launch")]
    Next,

    #[command(description = "help")]
    Help,
}

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "snake_case", description = "*Admin commands:*")]
enum AdminCommand {
    #[command(description = "help")]
    Help,

    #[command(description = "subscribers count")]
    SubscribersCount,
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
        UnauthorizedCommand::Start => match db.subscribe(msg.chat.id.0) {
            Ok(_) => {
                bot.send_message(
                    msg.chat.id,
                    markdown::escape("Subscribed, standby for notifications!"),
                )
                .reply_to_message_id(msg.id)
                .await?;
                let notify_up_to = Utc::now() + Duration::try_days(2).unwrap();
                for launch in db.get_launches().unwrap_or_default() {
                    let Some(t0) = launch.t0 else {
                        continue;
                    };
                    if t0 > notify_up_to {
                        continue;
                    }
                    let _ = launch_notify(&bot, &db, &launch, msg.chat.id.0, None).await;
                }
            }
            Err(err) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Error subscribing:\n```\n{}\n```",
                        markdown::escape(&format!("{:?}", err))
                    ),
                )
                .reply_to_message_id(msg.id)
                .await?;
            }
        },
        UnauthorizedCommand::Stop => match db.unsubscribe(msg.chat.id.0) {
            Ok(_) => {
                bot.send_message(msg.chat.id, "Unsubscribed")
                    .reply_to_message_id(msg.id)
                    .await?;
            }
            Err(err) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Error unsubscribing:\n```\n{}\n```",
                        markdown::escape(&format!("{:?}", err))
                    ),
                )
                .reply_to_message_id(msg.id)
                .await?;
            }
        },
        UnauthorizedCommand::Launches => {
            for launch in db.get_launches().unwrap_or_default() {
                if launch.t0.is_none() {
                    continue;
                }
                let _ = launch_notify(&bot, &db, &launch, msg.chat.id.0, Some(msg.id)).await;
            }
        }
        UnauthorizedCommand::Next => {
            let now = Utc::now();
            if let Some(launch) = db
                .get_launches()
                .unwrap_or_default()
                .iter()
                .filter(|l| {
                    if let Some(t0) = l.t0 {
                        t0 >= now
                    } else {
                        false
                    }
                })
                .min_by_key(|l| l.t0)
            {
                let _ = launch_notify(&bot, &db, launch, msg.chat.id.0, Some(msg.id)).await;
            }
        }
    }
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn command_handler(
    bot: MyBot,
    msg: Message,
    cmd: AdminCommand,
    db: Db,
) -> ResponseResult<()> {
    info!("handling: {:?}", cmd);
    match cmd {
        AdminCommand::Help => {
            let commands = [
                AdminCommand::descriptions().to_string(),
                UnauthorizedCommand::descriptions().to_string(),
            ];
            bot.send_message(msg.chat.id, commands.join("\\n\\n"))
                .reply_to_message_id(msg.id)
                .await?;
        }
        AdminCommand::SubscribersCount => {
            let count = db
                .subscribers_count()
                .expect("failed getting subscribers count");
            bot.send_message(msg.chat.id, format!("Total subscribers: {}", count))
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

pub async fn launches_notify(bot: &MyBot, db: &Db, launches: &[Launch]) -> Result<(), RLError> {
    for launch in launches {
        let Some(t0) = launch.t0 else {
            continue;
        };
        for chat_id in db.get_unnotified(launch.id, t0)? {
            launch_notify(bot, db, launch, chat_id, None).await?;
        }
    }

    Ok(())
}

pub async fn launch_notify(
    bot: &MyBot,
    db: &Db,
    launch: &Launch,
    chat_id: i64,
    msg_id: Option<MessageId>,
) -> Result<(), RLError> {
    let now = (Utc::now() + Duration::try_seconds(1).unwrap())
        .duration_round(TimeDelta::try_minutes(1).unwrap())
        .unwrap();
    let Some(t0) = launch.t0 else {
        return Ok(());
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

    if let Some(desc) = &launch.mission_description {
        let _ = write!(text, "\n\n{}", markdown::escape(desc));
    }

    if launch.suborbital {
        let _ = write!(text, "\n\nsuborbital");
    }

    info!("notifying {} about launch {}", chat_id, launch.id);
    let mut fut = bot.send_message(ChatId(chat_id), &text);
    if let Some(msg_id) = msg_id {
        fut = fut.reply_to_message_id(msg_id);
    }
    match fut.await {
        Ok(_) => {
            db.set_notified(chat_id, launch.id, t0)?;
        }
        Err(err) => {
            match err {
                RequestError::Api(ref api_err) => match api_err {
                    ApiError::BotBlocked
                    | ApiError::BotKicked
                    | ApiError::BotKickedFromSupergroup
                    | ApiError::ChatNotFound
                    | ApiError::UserNotFound
                    | ApiError::UserDeactivated
                    | ApiError::GroupDeactivated
                    | ApiError::CantTalkWithBots => {
                        warn!(
                            "unsubscribing {} from updates due to api error {}",
                            chat_id, api_err
                        );
                        db.unsubscribe(chat_id)?;
                    }
                    _ => {}
                },
                RequestError::MigrateToChatId(new_chat_id) => {
                    warn!(
                        "chat_id {} migrated to new chat_id {}",
                        chat_id, new_chat_id
                    );
                    db.replace_chat_id(chat_id, new_chat_id)?;
                }
                _ => {}
            }
            Err(err)?;
        }
    }

    Ok(())
}

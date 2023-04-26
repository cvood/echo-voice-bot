use std::{
    io::Write,
    path::PathBuf,
};

use args::Args;
use clap::Parser;
use teloxide::{prelude::*, types::InputFile};

use anyhow::Result;
use websocket::{Speaker, VOICE_XIAOYI};

mod args;
mod websocket;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    log::info!("Bot starting...");

    run().await;

    Ok(())
}


fn args() -> Args {
    Args::parse()
}

async fn run() {
    let bot = Bot::new("2132650312:AAFRxaYkuc002zgVN0lGdS_fYb6UetNmQcw");

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(text) = msg.text() {
            let speaker = Speaker::new(VOICE_XIAOYI, "+0%", "+0%");
            log::info!("Receive text message.");
            match speaker.say(text).await {
                Ok(data) => {
                    let mut file_path = args().data_path.unwrap_or(PathBuf::from("data"));
                    file_path.push(&format!("{}.webm", chrono::Utc::now().timestamp()));
                    let mut file = std::fs::File::create(&file_path)?;
                    file.write_all(&data)?;
                    bot.send_audio(msg.chat.id, InputFile::file(file_path)).await?;
                }
                Err(_) => ()
            }
        } else if let Some(_photo) = msg.photo() {
            log::info!("Receive photo message.");
            bot.send_message(msg.chat.id, "receive a photo !").await?;
        } else {
            log::info!("Receive other message.");
            bot.send_message(msg.chat.id, "unexpact message type !")
                .await?;
        }

        Ok(())
    })
    .await;
}

//! FSM bot with file-backed state dispenser

use vkontakte::dispatch::rules::{StateRule, TextRule};
use vkontakte::framework::{BotBuilder, Bot};
use vkontakte::state_group;
use vkontakte::VkError;

state_group! {
    pub enum Menu {
        Start = "start",
        Name = "name",
    }
}

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("VK_TOKEN").expect("VK_TOKEN");
    let group_id: i64 = std::env::var("VK_GROUP_ID").unwrap().parse().unwrap();

    let mut bot = BotBuilder::new(token)
        .group_id(group_id)
        .persistent_state_file(".vkontakte/fsm_state.json")
        .build()
        .await?;

    register_handlers(&mut bot);
    bot.run_polling().await
}

fn register_handlers(bot: &mut Bot) {
    bot.on()
        .message(Box::new(TextRule::new("/start", false)))
        .handle(|msg, _| async move {
            msg.set_state(Menu::Name).await?;
            msg.answer("Your name?").await?;
            Ok(None)
        });

    bot.on()
        .message(Box::new(StateRule::new(Menu::Name.as_str())))
        .handle(|msg, _| async move {
            let name = msg.text.clone();
            msg.set_state(Menu::Start).await?;
            msg.answer(&format!("Hello, {name}!")).await?;
            Ok(None)
        });
}

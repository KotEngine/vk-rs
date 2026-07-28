//! FSM bot with state dispenser and typed state group

use vkontakte::prelude::*;
use vkontakte::state_group;

state_group! {
    pub enum MenuState {
        Start = "start",
        Info = "info",
    }
}

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("VK_TOKEN").expect("set VK_TOKEN env var");
    let group_id: i64 = std::env::var("VK_GROUP_ID")
        .expect("set VK_GROUP_ID env var")
        .parse()
        .expect("VK_GROUP_ID must be integer");

    let mut bot = Bot::new(&token)?.with_group_id(group_id);

    bot.on()
        .message(Box::new(StateRule::none()))
        .handle(|msg, _ctx| async move {
            msg.answer("Привет! Напиши «info» или «buy»")
                .await
                .map(|v| Some(v))?;
            msg.set_state(MenuState::Start).await?;
            Ok(None)
        });

    bot.on()
        .message(Box::new(StateRule::new(MenuState::Start)))
        .rule(Box::new(TextRule::new("info", false)))
        .handle(|msg, _ctx| async move {
            msg.answer("Что тебя интересует? Книги или кино?")
                .await
                .map(|v| Some(v))?;
            msg.set_state(MenuState::Info).await?;
            Ok(None)
        });

    bot.on()
        .message(Box::new(StateRule::new(MenuState::Start)))
        .rule(Box::new(TextRule::new("buy", false)))
        .handle(|msg, _ctx| async move {
            msg.answer("Купить здесь: https://example.com")
                .await
                .map(|v| Some(v))
        });

    bot.on()
        .message(Box::new(StateRule::new(MenuState::Info)))
        .handle(|msg, _ctx| async move {
            msg.answer(&format!("Интересно! Ты написал: {}", msg.text))
                .await
                .map(|v| Some(v))?;
            msg.delete_state().await?;
            Ok(None)
        });

    bot.run_polling().await
}

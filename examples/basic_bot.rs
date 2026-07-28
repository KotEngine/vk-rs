//! Basic bot with a single text handler

use vkontakte::prelude::*;

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
        .message(Box::new(TextRule::new("hello", false)))
        .handle(|msg, _ctx| async move {
            msg.answer("world").await.map(|v| Some(v))
        });

    bot.run_polling().await
}

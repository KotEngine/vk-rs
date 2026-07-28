//! Callback API webhook server

use vkontakte::prelude::*;

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("VK_TOKEN").expect("set VK_TOKEN");
    let group_id: i64 = std::env::var("VK_GROUP_ID")
        .expect("set VK_GROUP_ID")
        .parse()
        .expect("VK_GROUP_ID must be integer");
    let secret = std::env::var("VK_CALLBACK_SECRET").expect("set VK_CALLBACK_SECRET");
    let confirmation = std::env::var("VK_CALLBACK_CONFIRMATION").expect("set VK_CALLBACK_CONFIRMATION");
    let server_url = std::env::var("VK_CALLBACK_URL").expect("set VK_CALLBACK_URL");

    let mut bot = Bot::new(&token)?.with_group_id(group_id);

    bot.on()
        .message(Box::new(TextRule::new("hello", false)))
        .handle(|msg, _ctx| async move { msg.answer("world").await.map(|v| Some(v)) });

    let config = CallbackConfig::new(group_id, secret, confirmation, server_url)
        .with_listen("0.0.0.0", 8080);

    let callback = BotCallback::new(config.clone(), bot.api.clone());
    callback.register_server().await?;

    bot.run_callback(config).await
}

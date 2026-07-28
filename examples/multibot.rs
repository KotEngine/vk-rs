//! Run two bots in parallel

use vkontakte::prelude::*;

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token_a = std::env::var("VK_TOKEN_A").expect("set VK_TOKEN_A");
    let token_b = std::env::var("VK_TOKEN_B").expect("set VK_TOKEN_B");
    let group_a: i64 = std::env::var("VK_GROUP_ID_A")
        .expect("set VK_GROUP_ID_A")
        .parse()
        .expect("VK_GROUP_ID_A must be integer");
    let group_b: i64 = std::env::var("VK_GROUP_ID_B")
        .expect("set VK_GROUP_ID_B")
        .parse()
        .expect("VK_GROUP_ID_B must be integer");

    let mut bot_a = Bot::new(&token_a)?.with_group_id(group_a);
    bot_a
        .on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|msg, _ctx| async move { msg.answer("pong A").await.map(|v| Some(v)) });

    let mut bot_b = Bot::new(&token_b)?.with_group_id(group_b);
    bot_b
        .on()
        .message(Box::new(TextRule::new("ping", false)))
        .handle(|msg, _ctx| async move { msg.answer("pong B").await.map(|v| Some(v)) });

    run_multibot(vec![bot_a, bot_b]).await
}

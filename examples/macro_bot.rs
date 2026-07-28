//! Bot with `#[on_message]` proc macro handlers (requires `macros` feature)

use std::collections::HashMap;

use vkontakte::on_message;
use vkontakte::prelude::*;
use serde_json::Value;

#[on_message(text = "hello")]
async fn hello(msg: MessageMin, _: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    msg.answer("world from macro!").await.map(|v| Some(v))
}

#[on_message(command = "ping")]
async fn ping(msg: MessageMin, _: HashMap<String, Value>) -> DispatchResult<Option<Value>> {
    msg.answer("pong").await.map(|v| Some(v))
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

    register_hello(&mut bot);
    register_ping(&mut bot);

    bot.run_polling().await
}

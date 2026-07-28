//! User account bot with long poll

use vkontakte::framework::{UserBuilder, User};
use vkontakte::prelude::*;
use vkontakte::VkError;

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("VK_USER_TOKEN").expect("VK_USER_TOKEN");
    let user_id: i64 = std::env::var("VK_USER_ID")
        .expect("VK_USER_ID")
        .parse()
        .expect("VK_USER_ID must be integer");

    let mut user = UserBuilder::new(token)
        .user_id(user_id)
        .build()
        .await?;

    register_handlers(&mut user);
    user.run().await
}

fn register_handlers(user: &mut User) {
    user.on()
        .message(Box::new(TextRule::new("hello", false)))
        .handle(|msg, _| async move {
            msg.answer("hi from user token").await?;
            Ok(None)
        });
}

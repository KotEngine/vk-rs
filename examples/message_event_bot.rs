//! Bot handling inline keyboard `message_event` callbacks

use vkontakte::prelude::*;
use vkontakte::tools::keyboard::{ButtonAction, ButtonColor, Keyboard};

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();

    let token = std::env::var("VK_TOKEN").expect("set VK_TOKEN");
    let group_id: i64 = std::env::var("VK_GROUP_ID")
        .expect("set VK_GROUP_ID")
        .parse()
        .expect("VK_GROUP_ID must be integer");

    let mut bot = Bot::new(&token)?.with_group_id(group_id);

    bot.on()
        .message(Box::new(TextRule::new("keyboard", false)))
        .handle(|msg, _| async move {
            let mut kb = Keyboard::new(false, true);
            kb.add(
                ButtonAction::callback("Click", r#"{"action":"ping"}"#),
                Some(ButtonColor::Primary),
            );
            let mut params = std::collections::HashMap::new();
            params.insert("peer_id".to_string(), msg.peer_id.to_string());
            params.insert("message".to_string(), "Press the button".to_string());
            params.insert("keyboard".to_string(), kb.to_json());
            params.insert("random_id".to_string(), "0".to_string());
            msg.api.request("messages.send", &params).await?;
            Ok(None)
        });

    bot.on()
        .message_event(Box::new(PayloadRule::new(r#"{"action":"ping"}"#)))
        .handle(|ev, _| async move {
            ev.show_snackbar("pong").await?;
            Ok(None)
        });

    bot.run_polling().await
}

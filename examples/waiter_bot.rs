//! Wait for the next user message matching a rule

use std::sync::Arc;
use std::time::Duration;

use vkontakte::dispatch::rules::TextRule;
use vkontakte::framework::Bot;
use vkontakte::tools::waiter::WaiterMachine;
use vkontakte::VkError;

#[tokio::main]
async fn main() -> Result<(), VkError> {
    tracing_subscriber::fmt::init();
    let token = std::env::var("VK_TOKEN").expect("VK_TOKEN");
    let group_id: i64 = std::env::var("VK_GROUP_ID").unwrap().parse().unwrap();

    let waiter = Arc::new(WaiterMachine::new());
    let mut bot = Bot::new(&token)?
        .with_group_id(group_id)
        .with_waiter_machine(waiter.clone());

    bot.on()
        .message(Box::new(TextRule::new("quiz", false)))
        .handle({
            let waiter = waiter.clone();
            move |msg, _| {
                let waiter = waiter.clone();
                async move {
                    msg.answer("Reply with `answer` within 30s").await?;
                    let rules: Vec<Arc<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>>> =
                        vec![Arc::new(TextRule::new("answer", false))];
                    match waiter
                        .wait("message", msg.peer_id, rules, Some(Duration::from_secs(30)))
                        .await
                    {
                        Ok(_) => {
                            msg.answer("Correct!").await?;
                        }
                        Err(_) => {
                            msg.answer("Too late.").await?;
                        }
                    }
                    Ok(None)
                }
            }
        });

    bot.run_polling().await
}

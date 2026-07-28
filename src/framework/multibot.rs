//! Run multiple bot instances concurrently

use tokio::task::JoinSet;

use crate::exception::VkResult;
use crate::framework::Bot;

/// Run several bots in parallel; returns when all polling loops exit.
pub async fn run_multibot(bots: Vec<Bot>) -> VkResult<()> {
    let mut set = JoinSet::new();

    for mut bot in bots {
        set.spawn(async move { bot.run_polling().await });
    }

    while let Some(result) = set.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => {
                return Err(crate::exception::VkError::Internal(format!(
                    "multibot task panicked: {e}"
                )));
            }
        }
    }

    Ok(())
}

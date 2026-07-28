//! Fluent builder for `User` instances

use std::path::PathBuf;
use std::sync::Arc;

use crate::dispatch::dispenser::{FileStateDispenser, StateDispenser};
use crate::exception::{ErrorHandler, VkResult};
use crate::framework::{User, UserBlueprint};
use crate::tools::waiter::SharedWaiter;

pub struct UserBuilder {
    token: String,
    user_id: Option<i64>,
    state_file: Option<PathBuf>,
    custom_dispenser: Option<Arc<dyn StateDispenser>>,
    waiter: Option<SharedWaiter>,
    blueprints: Vec<UserBlueprint>,
    error_handler: Option<Box<dyn ErrorHandler>>,
}

impl UserBuilder {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            user_id: None,
            state_file: None,
            custom_dispenser: None,
            waiter: None,
            blueprints: Vec::new(),
            error_handler: None,
        }
    }

    pub fn user_id(mut self, id: i64) -> Self {
        self.user_id = Some(id);
        self
    }

    pub fn persistent_state_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.state_file = Some(path.into());
        self
    }

    pub fn state_dispenser(mut self, dispenser: Arc<dyn StateDispenser>) -> Self {
        self.custom_dispenser = Some(dispenser);
        self
    }

    pub fn waiter_machine(mut self, machine: SharedWaiter) -> Self {
        self.waiter = Some(machine);
        self
    }

    pub fn include_blueprint(mut self, bp: UserBlueprint) -> Self {
        self.blueprints.push(bp);
        self
    }

    pub fn error_handler(mut self, handler: Box<dyn ErrorHandler>) -> Self {
        self.error_handler = Some(handler);
        self
    }

    pub async fn build(mut self) -> VkResult<User> {
        let mut user = User::new(&self.token)?;

        if let Some(uid) = self.user_id {
            user = user.with_user_id(uid);
        }

        if let Some(path) = self.state_file.take() {
            let dispenser = Arc::new(FileStateDispenser::open(path).await?);
            user = user.with_state_dispenser(dispenser);
        } else if let Some(dispenser) = self.custom_dispenser.take() {
            user = user.with_state_dispenser(dispenser);
        }

        if let Some(waiter) = self.waiter.take() {
            user = user.with_waiter_machine(waiter);
        }

        if let Some(handler) = self.error_handler.take() {
            user.error_handler = handler;
        }

        for bp in self.blueprints {
            user.include(bp);
        }

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn user_builder_default() {
        let user = UserBuilder::new("token").user_id(1).build().await.unwrap();
        assert_eq!(user.user_id(), 1);
    }
}

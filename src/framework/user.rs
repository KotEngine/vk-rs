//! User account framework

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt;
use serde_json::Value;

use crate::api::{shared_api, Api};
use crate::dispatch::dispenser::{BuiltinStateDispenser, StateDispenser};
use crate::dispatch::middlewares::Middleware;
use crate::dispatch::router::DispatchRouter;
use crate::dispatch::views::UserMessageView;
use crate::dispatch::Router;
use crate::exception::{DefaultErrorHandler, ErrorHandler, VkResult};
use crate::framework::{Framework, UserBlueprint, UserLabeler, UserOn};
use crate::polling::{Polling, PollingConfig, UserPolling};
use crate::tools::ctx_storage::CtxStorage;
use crate::tools::waiter::{try_feed_message_waiters, SharedWaiter, WaiterMachine};

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type HookFn = Box<dyn Fn() -> BoxFuture + Send + Sync>;

/// Top-level user account framework
pub struct User {
    pub api: Arc<Api>,
    pub labeler: UserLabeler,
    pub state_dispenser: Arc<dyn StateDispenser>,
    pub error_handler: Box<dyn ErrorHandler>,
    polling: Option<UserPolling>,
    user_id: i64,
    router: Arc<DispatchRouter>,
    router_synced: bool,
    on_startup: Vec<HookFn>,
    on_shutdown: Vec<HookFn>,
    waiter_machine: SharedWaiter,
    pending_middleware: Vec<Box<dyn Middleware<Value>>>,
    pub ctx_storage: Arc<CtxStorage>,
}

impl User {
    pub fn new(token: &str) -> VkResult<Self> {
        let api = Arc::new(Api::new(token)?);
        Ok(Self {
            api,
            labeler: UserLabeler::new(),
            state_dispenser: Arc::new(BuiltinStateDispenser::new()),
            error_handler: Box::new(DefaultErrorHandler::new()),
            polling: None,
            user_id: 0,
            router: Arc::new(DispatchRouter::new()),
            router_synced: false,
            on_startup: Vec::new(),
            on_shutdown: Vec::new(),
            waiter_machine: Arc::new(WaiterMachine::new()),
            pending_middleware: Vec::new(),
            ctx_storage: Arc::new(CtxStorage::new()),
        })
    }

    pub async fn run(&mut self) -> VkResult<()> {
        self.run_polling().await
    }

    pub fn waiter_machine(&self) -> SharedWaiter {
        self.waiter_machine.clone()
    }

    pub fn with_waiter_machine(mut self, machine: SharedWaiter) -> Self {
        self.waiter_machine = machine;
        self
    }

    pub fn with_user_id(mut self, user_id: i64) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn user_id(&self) -> i64 {
        self.user_id
    }

    pub fn with_state_dispenser(mut self, dispenser: Arc<dyn StateDispenser>) -> Self {
        self.state_dispenser = dispenser;
        self.router_synced = false;
        self
    }

    pub fn add_startup_hook<F, Fut>(&mut self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_startup
            .push(Box::new(move || Box::pin(hook())));
    }

    pub fn add_shutdown_hook<F, Fut>(&mut self, hook: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_shutdown
            .push(Box::new(move || Box::pin(hook())));
    }

    pub fn router(&self) -> Arc<DispatchRouter> {
        self.router.clone()
    }

    /// Register a middleware. Must be called before the router is finalized.
    pub fn use_middleware<M>(&mut self, middleware: M)
    where
        M: Middleware<Value> + 'static,
    {
        self.pending_middleware.push(Box::new(middleware));
        self.router_synced = false;
    }

    /// Absorb a [`UserBlueprint`] — handlers, blueprint-local middleware and
    /// nested blueprints are flattened into this user framework.
    pub fn mount(&mut self, blueprint: UserBlueprint) {
        self.include(blueprint);
    }

    pub fn include(&mut self, mut blueprint: UserBlueprint) {
        for handler in blueprint.labeler.cloned_message_handlers() {
            self.labeler.push_message_handler(handler);
        }
        for handler in blueprint.labeler.cloned_raw_handlers() {
            self.labeler.push_raw_handler(handler);
        }
        for mw in blueprint.take_middleware() {
            self.pending_middleware.push(mw);
        }
        for nested in blueprint.take_nested() {
            self.include(nested);
        }
        self.router_synced = false;
    }

    pub fn on(&mut self) -> UserOn<'_> {
        UserOn {
            labeler: &mut self.labeler,
        }
    }

    pub fn sync_router(&mut self) {
        let mut router = DispatchRouter::new();

        let mut message_view = UserMessageView::new(self.api.clone())
            .with_state_dispenser(self.state_dispenser.clone());
        for handler in self.labeler.cloned_message_handlers() {
            message_view.register_handler(handler);
        }
        router.add_view(Box::new(message_view));

        router.register_middleware(Box::new(
            crate::dispatch::middlewares::WaiterMiddleware::for_messages(
                self.waiter_machine.clone(),
            ),
        ));

        for handler in self.labeler.cloned_raw_handlers() {
            router.register_handler(handler);
        }

        for mw in self.pending_middleware.drain(..) {
            router.register_middleware(mw);
        }

        self.router = Arc::new(router);
        self.router_synced = true;
    }

    fn setup_polling(&mut self) {
        let api = shared_api(self.api.clone());
        let mut config = PollingConfig::default();
        if self.user_id != 0 {
            config.ts_file = Some(format!(".vkontakte/polling/user_{}.ts", self.user_id));
        }
        self.polling = Some(UserPolling::with_config(api, self.user_id, config));
    }

    async fn run_startup_hooks(&self) {
        for hook in &self.on_startup {
            hook().await;
        }
    }

    async fn run_shutdown_hooks(&self) {
        for hook in &self.on_shutdown {
            hook().await;
        }
    }

    pub async fn run_polling(&mut self) -> VkResult<()> {
        if !self.router_synced {
            self.sync_router();
        }

        self.run_startup_hooks().await;

        if self.polling.is_none() {
            self.setup_polling();
        }

        let polling = self.polling.as_ref().expect("polling configured");
        let mut stream = polling.listen();
        let router = self.router.clone();
        let api = self.api.clone();
        let state_dispenser = self.state_dispenser.clone();
        let error_handler = &self.error_handler;

        let waiter = self.waiter_machine.clone();
        let result = async {
            while let Some(event) = stream.next().await {
                let _ = try_feed_message_waiters(&waiter, "message", &event).await;
                let route_result = router
                    .route(&event, &api, Some(state_dispenser.as_ref()))
                    .await;
                if let Err(e) = route_result {
                    let _ = error_handler.handle(&e).await;
                }
            }
            Ok(())
        }
        .await;

        self.run_shutdown_hooks().await;
        result
    }
}

#[async_trait::async_trait]
impl Framework for User {
    async fn run_polling(&self) -> VkResult<()> {
        Err(crate::exception::VkError::Internal(
            "Use User::run_polling on &mut User".to_string(),
        ))
    }

    async fn on_startup(&self) {}
    async fn on_shutdown(&self) {}
}

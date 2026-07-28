//! Main bot framework

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::StreamExt;

use crate::api::{shared_api, Api};
use crate::callback::CallbackConfig;
use crate::dispatch::dispenser::{BuiltinStateDispenser, StateDispenser};
use crate::dispatch::middlewares::Middleware;
use crate::dispatch::router::DispatchRouter;
use crate::dispatch::views::{BotMessageView, MessageEventView, RawEventView};
use crate::dispatch::Router;
use crate::exception::{DefaultErrorHandler, ErrorHandler, VkResult};
use crate::framework::routes::{RouteInfo, RouteKind};
use crate::framework::{BotBlueprint, BotLabeler, BotOn, Framework};
use crate::polling::{BotPolling, Polling, PollingConfig};
use crate::tools::ctx_storage::CtxStorage;
use crate::tools::waiter::{try_feed_message_waiters, SharedWaiter, WaiterMachine};
use serde_json::Value;

type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
type HookFn = Box<dyn Fn() -> BoxFuture + Send + Sync>;

/// Top-level bot framework
pub struct Bot {
    pub api: Arc<Api>,
    pub labeler: BotLabeler,
    pub state_dispenser: Arc<dyn StateDispenser>,
    pub error_handler: Box<dyn ErrorHandler>,
    polling: Option<BotPolling>,
    group_id: i64,
    router: Arc<DispatchRouter>,
    router_synced: bool,
    on_startup: Vec<HookFn>,
    on_shutdown: Vec<HookFn>,
    waiter_machine: SharedWaiter,
    /// Middleware registered via [`Bot::use_middleware`] before the router is
    /// finalized. Drained into the frozen router by [`Bot::sync_router`].
    pending_middleware: Vec<Box<dyn Middleware<Value>>>,
    pub ctx_storage: Arc<CtxStorage>,
    /// Snapshot of what `sync_router` registered, for [`Bot::dump_routes`].
    routes: Vec<RouteInfo>,
    /// Names of blueprints mounted so far (unnamed ones are skipped).
    mounted_blueprints: Vec<String>,
}

impl Bot {
    pub fn new(token: &str) -> VkResult<Self> {
        let api = Arc::new(Api::new(token)?);
        Ok(Self {
            api,
            labeler: BotLabeler::new(),
            state_dispenser: Arc::new(BuiltinStateDispenser::new()),
            error_handler: Box::new(DefaultErrorHandler::new()),
            polling: None,
            group_id: 0,
            router: Arc::new(DispatchRouter::new()),
            router_synced: false,
            on_startup: Vec::new(),
            on_shutdown: Vec::new(),
            waiter_machine: Arc::new(WaiterMachine::new()),
            pending_middleware: Vec::new(),
            ctx_storage: Arc::new(CtxStorage::new()),
            routes: Vec::new(),
            mounted_blueprints: Vec::new(),
        })
    }

    /// Alias for [`Bot::run_polling`]
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

    pub fn with_group_id(mut self, group_id: i64) -> Self {
        self.group_id = group_id;
        self
    }

    pub fn group_id(&self) -> i64 {
        self.group_id
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

    /// Number of middleware queued via [`Bot::use_middleware`] or mounted
    /// blueprints that have not yet been folded into the router.
    pub fn pending_middleware_len(&self) -> usize {
        self.pending_middleware.len()
    }

    /// Register a middleware.
    ///
    /// Middleware must be added **before** the router is finalized (i.e. before
    /// `run_polling` / `run_callback`). They are drained into the frozen router
    /// the next time [`Bot::sync_router`] runs.
    pub fn use_middleware<M>(&mut self, middleware: M)
    where
        M: Middleware<Value> + 'static,
    {
        self.pending_middleware.push(Box::new(middleware));
        self.router_synced = false;
    }

    /// Absorb a [`BotBlueprint`] — handlers, blueprint-local middleware and
    /// nested blueprints are flattened into this bot.
    ///
    /// This is the primary composition entry point for modular bots. `mount`
    /// is the idiomatic alias for [`Bot::include`].
    pub fn mount(&mut self, blueprint: BotBlueprint) {
        self.include(blueprint);
    }

    pub fn include(&mut self, mut blueprint: BotBlueprint) {
        if !blueprint.name.is_empty() {
            self.mounted_blueprints.push(blueprint.name.clone());
        }
        for handler in blueprint.labeler.cloned_message_handlers() {
            self.labeler.push_message_handler(handler);
        }
        for handler in blueprint.labeler.cloned_message_event_handlers() {
            self.labeler.push_message_event_handler(handler);
        }
        for (event_type, handlers) in blueprint.labeler.cloned_raw_handlers() {
            for handler in handlers {
                self.labeler.push_raw_handler(event_type.clone(), handler);
            }
        }
        for handler in blueprint.labeler.cloned_value_handlers() {
            self.labeler.value_handlers.push(handler);
        }
        for mw in blueprint.take_middleware() {
            self.pending_middleware.push(mw);
        }
        // Recursively flatten nested blueprints.
        for nested in blueprint.take_nested() {
            self.include(nested);
        }
        self.router_synced = false;
    }

    pub fn on(&mut self) -> BotOn<'_> {
        BotOn {
            labeler: &mut self.labeler,
        }
    }

    /// Build the immutable router from the labeler and pending middleware.
    ///
    /// After this the router is frozen inside an [`Arc`] and shared across all
    /// concurrent dispatch tasks without any locking.
    pub fn sync_router(&mut self) {
        let mut router = DispatchRouter::new();
        let mut routes = Vec::new();

        let mut message_view = BotMessageView::new(self.api.clone())
            .with_state_dispenser(self.state_dispenser.clone())
            .with_storage(self.ctx_storage.clone());
        for handler in self.labeler.cloned_message_handlers() {
            routes.push(RouteInfo::new(RouteKind::Message, handler.describe()));
            message_view.register_handler(handler);
        }
        router.add_view(Box::new(message_view));

        let mut message_event_view =
            MessageEventView::new(self.api.clone()).with_storage(self.ctx_storage.clone());
        for handler in self.labeler.cloned_message_event_handlers() {
            routes.push(RouteInfo::new(RouteKind::MessageEvent, handler.describe()));
            message_event_view.register_handler(handler);
        }
        router.add_view(Box::new(message_event_view));

        let mut raw_view = RawEventView::new();
        for (event_type, handlers) in self.labeler.cloned_raw_handlers() {
            for handler in handlers {
                routes.push(
                    RouteInfo::new(RouteKind::RawEvent, handler.describe())
                        .with_event_type(&event_type),
                );
                raw_view.register(&event_type, handler);
            }
        }
        router.add_view(Box::new(raw_view));

        router.register_middleware(Box::new(
            crate::dispatch::middlewares::WaiterMiddleware::for_messages(
                self.waiter_machine.clone(),
            ),
        ));

        for handler in self.labeler.cloned_value_handlers() {
            routes.push(RouteInfo::new(RouteKind::RawValue, handler.describe()));
            router.register_handler(handler);
        }

        for mw in self.pending_middleware.drain(..) {
            router.register_middleware(mw);
        }

        self.routes = routes;
        self.router = Arc::new(router);
        self.router_synced = true;
    }

    /// Every route registered by the last [`Bot::sync_router`].
    pub fn routes(&self) -> &[RouteInfo] {
        &self.routes
    }

    /// Names of blueprints mounted onto this bot.
    pub fn mounted_blueprints(&self) -> &[String] {
        &self.mounted_blueprints
    }

    /// Routing table as text, syncing the router first if needed.
    pub fn format_routes(&mut self) -> String {
        if !self.router_synced {
            self.sync_router();
        }
        crate::framework::routes::format_routes(&self.routes, &self.mounted_blueprints)
    }

    /// Print the routing table to stdout.
    ///
    /// Syncs the router first, so it is safe to call right after registration.
    pub fn dump_routes(&mut self) {
        println!("{}", self.format_routes());
    }

    fn setup_polling(&mut self) {
        let api = shared_api(self.api.clone());
        let mut config = PollingConfig::default();
        if self.group_id != 0 {
            config.ts_file = Some(format!(".vkontakte/polling/bot_{}.ts", self.group_id));
        }
        self.polling = Some(BotPolling::with_config(api, self.group_id, config));
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

    pub async fn run_callback(&mut self, config: CallbackConfig) -> VkResult<()> {
        if !self.router_synced {
            self.sync_router();
        }

        self.run_startup_hooks().await;

        let callback = crate::callback::BotCallback::new(config, self.api.clone());
        let result = callback
            .run_with_waiter(
                self.router.clone(),
                self.state_dispenser.clone(),
                self.waiter_machine.clone(),
            )
            .await;

        self.run_shutdown_hooks().await;
        result
    }
}

#[async_trait::async_trait]
impl Framework for Bot {
    async fn run_polling(&self) -> VkResult<()> {
        Err(crate::exception::VkError::Internal(
            "Use Bot::run_polling on &mut Bot".to_string(),
        ))
    }

    async fn on_startup(&self) {}
    async fn on_shutdown(&self) {}
}

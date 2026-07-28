//! Turning plain async functions into handlers via their argument types.
//!
//! Each argument must implement [`FromEventContext`]. The `Args` type parameter
//! carries the argument tuple, which is what lets one trait cover every arity
//! without overlapping impls.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use super::{ExtractContext, FromEventContext};
use crate::dispatch::handlers::{evaluate_rules, Handler};
use crate::dispatch::rules::Rule;
use crate::dispatch::{DispatchResult, EventContext, RuleResult};
use crate::exception::VkError;
use crate::tools::ctx_storage::CtxStorage;
use crate::tools::mini_types::{MessageEventMin, MessageMin};

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// An async function whose arguments are all extractors.
///
/// Taking `self: Arc<Self>` lets the returned future own the function, so it is
/// `'static` without any lifetime tricks.
pub trait ExtractHandler<Args>: Send + Sync + 'static {
    fn call(self: Arc<Self>, ctx: ExtractContext) -> BoxFuture<DispatchResult<Option<Value>>>;
}

macro_rules! impl_extract_handler {
    ( $( $arg:ident ),* ) => {
        #[allow(non_snake_case, unused_variables)]
        impl<F, Fut, $( $arg, )*> ExtractHandler<( $( $arg, )* )> for F
        where
            F: Fn( $( $arg, )* ) -> Fut + Send + Sync + 'static,
            Fut: Future<Output = DispatchResult<Option<Value>>> + Send + 'static,
            $( $arg: FromEventContext + Send + 'static, )*
        {
            fn call(
                self: Arc<Self>,
                ctx: ExtractContext,
            ) -> BoxFuture<DispatchResult<Option<Value>>> {
                Box::pin(async move {
                    // Extraction is async and can fail, so it happens here rather
                    // than at registration time.
                    $( let $arg = <$arg as FromEventContext>::from_ctx(&ctx).await?; )*
                    (self)( $( $arg, )* ).await
                })
            }
        }
    };
}

impl_extract_handler!(A1);
impl_extract_handler!(A1, A2);
impl_extract_handler!(A1, A2, A3);
impl_extract_handler!(A1, A2, A3, A4);
impl_extract_handler!(A1, A2, A3, A4, A5);
impl_extract_handler!(A1, A2, A3, A4, A5, A6);
impl_extract_handler!(A1, A2, A3, A4, A5, A6, A7);
impl_extract_handler!(A1, A2, A3, A4, A5, A6, A7, A8);

/// [`Handler`] wrapper around an extractor-based function.
///
/// Generic over the event type the surrounding view dispatches (`MessageMin`,
/// `MessageEventMin`, or raw `Value`) — rules and extractors both read the raw
/// update out of [`EventContext`], so the typed event itself is unused.
pub struct ExtractFuncHandler<E> {
    rules: Vec<Box<dyn Rule<Value>>>,
    func: Arc<dyn Fn(ExtractContext) -> BoxFuture<DispatchResult<Option<Value>>> + Send + Sync>,
    _event: std::marker::PhantomData<fn() -> E>,
}

impl<E> ExtractFuncHandler<E> {
    pub fn new<H, Args>(rules: Vec<Box<dyn Rule<Value>>>, handler: H) -> Self
    where
        H: ExtractHandler<Args>,
        Args: 'static,
    {
        let handler = Arc::new(handler);
        Self {
            rules,
            func: Arc::new(move |ctx| handler.clone().call(ctx)),
            _event: std::marker::PhantomData,
        }
    }

    /// Rule descriptions, for router introspection.
    fn describe_rules(&self) -> String {
        if self.rules.is_empty() {
            return "[no rules]".to_string();
        }
        format!(
            "[{}]",
            self.rules
                .iter()
                .map(|r| r.description())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    async fn run(&self, ctx: &mut EventContext) -> DispatchResult<Option<Value>> {
        let rule_context = match evaluate_rules(&self.rules, &ctx.event).await {
            RuleResult::Fail => return Ok(None),
            RuleResult::Pass => HashMap::new(),
            RuleResult::Context(c) => c,
        };

        let Some(api) = ctx.api.clone() else {
            return Err(VkError::Validation(
                "extractor handler dispatched without an API client".to_string(),
            )
            .into());
        };

        // Rule context wins over anything middleware already put in place.
        let mut merged = ctx.context_update.clone();
        merged.extend(rule_context);

        let storage = ctx
            .storage
            .clone()
            .unwrap_or_else(|| Arc::new(CtxStorage::new()));

        let extract_ctx = ExtractContext::new(ctx.event.clone(), api, storage, merged);

        (self.func)(extract_ctx).await
    }
}

#[async_trait]
impl Handler<Value> for ExtractFuncHandler<Value> {
    async fn handle(
        &self,
        _event: &Value,
        ctx: &mut EventContext,
    ) -> DispatchResult<Option<Value>> {
        self.run(ctx).await
    }

    fn rules(&self) -> &[Box<dyn Rule<Value>>] {
        &self.rules
    }

    fn describe(&self) -> String {
        self.describe_rules()
    }
}

#[async_trait]
impl Handler<MessageMin> for ExtractFuncHandler<MessageMin> {
    async fn handle(
        &self,
        _event: &MessageMin,
        ctx: &mut EventContext,
    ) -> DispatchResult<Option<Value>> {
        self.run(ctx).await
    }

    fn rules(&self) -> &[Box<dyn Rule<MessageMin>>] {
        &[]
    }

    fn describe(&self) -> String {
        self.describe_rules()
    }
}

#[async_trait]
impl Handler<MessageEventMin> for ExtractFuncHandler<MessageEventMin> {
    async fn handle(
        &self,
        _event: &MessageEventMin,
        ctx: &mut EventContext,
    ) -> DispatchResult<Option<Value>> {
        self.run(ctx).await
    }

    fn rules(&self) -> &[Box<dyn Rule<MessageEventMin>>] {
        &[]
    }

    fn describe(&self) -> String {
        self.describe_rules()
    }
}

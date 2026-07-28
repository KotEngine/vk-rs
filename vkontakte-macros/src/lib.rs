//! Proc macros for vkontakte

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, ItemFn, LitBool, LitInt, LitStr, Variant};

/// Attach a message handler rule to an async function.
///
/// Every argument contributes one rule and all of them must match:
///
/// ```ignore
/// #[on_message(state = "menu:main", text = "профиль")]
/// #[on_message(command = "buy", cooldown_secs = 5)]
/// ```
#[proc_macro_attribute]
pub fn on_message(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr: proc_macro2::TokenStream = attr.into();
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    let mut text: Option<String> = None;
    let mut ignore_case = false;
    let mut command: Option<String> = None;
    let mut regex: Option<String> = None;
    let mut payload: Option<String> = None;
    let mut state: Option<String> = None;
    let mut no_state = false;
    let mut cooldown_secs: Option<u64> = None;
    let mut cooldown_scope: Option<String> = None;
    let mut from_chat: Option<bool> = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("text") {
            text = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("command") {
            command = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("regex") {
            regex = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("payload") {
            payload = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("state") {
            state = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("ignore_case") {
            ignore_case = meta.value()?.parse::<LitBool>()?.value;
        } else if meta.path.is_ident("no_state") {
            no_state = meta.value()?.parse::<LitBool>()?.value;
        } else if meta.path.is_ident("cooldown_secs") {
            cooldown_secs = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
        } else if meta.path.is_ident("cooldown_scope") {
            cooldown_scope = Some(meta.value()?.parse::<LitStr>()?.value());
        } else if meta.path.is_ident("from_chat") {
            from_chat = Some(meta.value()?.parse::<LitBool>()?.value);
        } else {
            return Err(meta.error(
                "unknown on_message attribute (expected one of: text, command, regex, payload, \
                 state, no_state, ignore_case, cooldown_secs, cooldown_scope, from_chat)",
            ));
        }
        Ok(())
    });

    if let Err(e) = syn::parse::Parser::parse2(parser, attr) {
        return e.to_compile_error().into();
    }

    if state.is_some() && no_state {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "`state` and `no_state` cannot be combined",
        )
        .to_compile_error()
        .into();
    }

    let mut rules: Vec<proc_macro2::TokenStream> = Vec::new();

    // A cooldown is checked first so a rejected call never runs the other rules.
    if let Some(secs) = cooldown_secs {
        let scope = cooldown_scope.as_deref().unwrap_or("user");
        let ctor = match scope {
            "user" => quote!(per_user),
            "peer" | "chat" => quote!(per_peer),
            "global" => quote!(global),
            other => {
                return syn::Error::new_spanned(
                    &func.sig.ident,
                    format!(
                        "unknown cooldown_scope {other:?} (expected \"user\", \"peer\" or \"global\")"
                    ),
                )
                .to_compile_error()
                .into();
            }
        };
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::CooldownRule::#ctor(
                std::time::Duration::from_secs(#secs)
            ))
        });
    } else if cooldown_scope.is_some() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "`cooldown_scope` requires `cooldown_secs`",
        )
        .to_compile_error()
        .into();
    }

    if let Some(st) = &state {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::StateRule::new(#st))
        });
    } else if no_state {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::StateRule::none())
        });
    }

    if let Some(chat) = from_chat {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::PeerRule::new(#chat))
        });
    }

    if let Some(cmd) = &command {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::CommandRule::new(#cmd, vec!["/", "!"], None))
        });
    }

    if let Some(re) = &regex {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::RegexRule::new(#re))
        });
    }

    if let Some(p) = &payload {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::PayloadRule::new(#p))
        });
    }

    if let Some(t) = &text {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::TextRule::new(#t, #ignore_case))
        });
    }

    if rules.is_empty() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "on_message needs at least one rule (e.g. `text = \"...\"` or `command = \"...\"`)",
        )
        .to_compile_error()
        .into();
    }

    let first = &rules[0];
    let rest = &rules[1..];

    let register_name = syn::Ident::new(&format!("register_{func_name}"), func_name.span());

    quote! {
        #func

        #[allow(dead_code)]
        pub fn #register_name(bot: &mut vkontakte::framework::Bot) {
            let __first: Box<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>> = #first;
            bot.on()
                .message(__first)
                #(.rule({
                    let __r: Box<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>> = #rest;
                    __r
                }))*
                .handle(|msg, ctx| async move {
                    #func_name(msg, ctx).await
                });
        }
    }
    .into()
}

/// Derive FSM state repr helpers for an enum.
#[proc_macro_derive(StateGroup, attributes(state_value))]
pub fn state_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let group = name.to_string();

    let variants = match input.data {
        Data::Enum(data) => data.variants,
        _ => {
            return syn::Error::new_spanned(name, "StateGroup can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    let mut as_str_arms = Vec::new();

    for variant in variants {
        let v_ident = &variant.ident;
        let value = variant_value(&variant);
        as_str_arms.push(quote! {
            Self::#v_ident => vkontakte::tools::fsm::make_state_repr(#group, #value),
        });
    }

    quote! {
        impl #name {
            pub fn group_name() -> &'static str {
                #group
            }

            pub fn as_str(&self) -> String {
                match self {
                    #(#as_str_arms)*
                }
            }
        }

        impl From<#name> for String {
            fn from(value: #name) -> String {
                value.as_str()
            }
        }

        impl vkontakte::tools::fsm::StateGroupValue for #name {
            fn group_name() -> &'static str {
                #group
            }

            fn as_str(&self) -> String {
                match self {
                    #(#as_str_arms)*
                }
            }
        }
    }
    .into()
}

fn variant_value(variant: &Variant) -> String {
    for attr in &variant.attrs {
        if attr.path().is_ident("state_value") {
            if let Ok(lit) = attr.parse_args::<LitStr>() {
                return lit.value();
            }
        }
    }

    to_snake_case(&variant.ident.to_string())
}

/// Attach a `message_event` (callback button) handler to an async function.
///
/// ```ignore
/// #[on_message_event(payload = r#"{"action": "buy"}"#)]
/// #[on_message_event(payload_contains = "action", payload_value = "buy")]
/// ```
///
/// A bare string literal is also accepted and treated as `payload`.
#[proc_macro_attribute]
pub fn on_message_event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr: proc_macro2::TokenStream = attr.into();
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    let mut payload: Option<String> = None;
    let mut payload_contains: Option<String> = None;
    let mut payload_value: Option<String> = None;
    let mut cooldown_secs: Option<u64> = None;

    if !attr.is_empty() {
        // Bare `#[on_message_event("...")]` stays valid.
        if let Ok(lit) = syn::parse2::<LitStr>(attr.clone()) {
            payload = Some(lit.value());
        } else {
            let parser = syn::meta::parser(|meta| {
                if meta.path.is_ident("payload") {
                    payload = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("payload_contains") {
                    payload_contains = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("payload_value") {
                    payload_value = Some(meta.value()?.parse::<LitStr>()?.value());
                } else if meta.path.is_ident("cooldown_secs") {
                    cooldown_secs = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
                } else {
                    return Err(meta.error(
                        "unknown on_message_event attribute (expected one of: payload, \
                         payload_contains, payload_value, cooldown_secs)",
                    ));
                }
                Ok(())
            });
            if let Err(e) = syn::parse::Parser::parse2(parser, attr) {
                return e.to_compile_error().into();
            }
        }
    }

    if payload_value.is_some() && payload_contains.is_none() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "`payload_value` requires `payload_contains`",
        )
        .to_compile_error()
        .into();
    }

    let mut rules: Vec<proc_macro2::TokenStream> = Vec::new();

    if let Some(secs) = cooldown_secs {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::CooldownRule::per_user(
                std::time::Duration::from_secs(#secs)
            ))
        });
    }

    if let Some(key) = &payload_contains {
        // No `payload_value` means "the key just has to be present".
        rules.push(match &payload_value {
            Some(v) => quote! {
                Box::new(vkontakte::dispatch::rules::PayloadContainsRule::new(
                    #key,
                    serde_json::Value::String(#v.to_string()),
                ))
            },
            None => quote! {
                Box::new(vkontakte::dispatch::rules::PayloadHasKeyRule::new(#key))
            },
        });
    }

    if let Some(p) = &payload {
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::PayloadRule::new(#p))
        });
    }

    if rules.is_empty() {
        // Bare `#[on_message_event]` matches every callback event.
        rules.push(quote! {
            Box::new(vkontakte::dispatch::rules::FuncRule::new(|_| {
                vkontakte::dispatch::RuleResult::Pass
            }))
        });
    }

    let first = &rules[0];
    let rest = &rules[1..];

    let register_name = syn::Ident::new(&format!("register_{func_name}"), func_name.span());

    quote! {
        #func

        #[allow(dead_code)]
        pub fn #register_name(bot: &mut vkontakte::framework::Bot) {
            let __first: Box<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>> = #first;
            bot.on()
                .message_event(__first)
                #(.rule({
                    let __r: Box<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>> = #rest;
                    __r
                }))*
                .handle(|ev, ctx| async move {
                    #func_name(ev, ctx).await
                });
        }
    }
    .into()
}

/// Attach a raw VK event handler to an async function.
///
/// ```ignore
/// #[on_raw_event(event_type = "wall_post_new")]
/// #[on_raw_event("group_join")]
/// ```
#[proc_macro_attribute]
pub fn on_raw_event(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr: proc_macro2::TokenStream = attr.into();
    let func = parse_macro_input!(item as ItemFn);
    let func_name = &func.sig.ident;

    let mut event_type: Option<String> = None;
    let mut cooldown_secs: Option<u64> = None;

    if attr.is_empty() {
        return syn::Error::new_spanned(
            &func.sig.ident,
            "on_raw_event needs an event type, e.g. `#[on_raw_event(event_type = \"wall_post_new\")]`",
        )
        .to_compile_error()
        .into();
    }

    // Bare `#[on_raw_event("wall_post_new")]` stays valid.
    if let Ok(lit) = syn::parse2::<LitStr>(attr.clone()) {
        event_type = Some(lit.value());
    } else {
        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("event_type") {
                event_type = Some(meta.value()?.parse::<LitStr>()?.value());
            } else if meta.path.is_ident("cooldown_secs") {
                cooldown_secs = Some(meta.value()?.parse::<LitInt>()?.base10_parse()?);
            } else {
                return Err(meta.error(
                    "unknown on_raw_event attribute (expected one of: event_type, cooldown_secs)",
                ));
            }
            Ok(())
        });
        if let Err(e) = syn::parse::Parser::parse2(parser, attr) {
            return e.to_compile_error().into();
        }
    }

    let Some(event_type) = event_type else {
        return syn::Error::new_spanned(&func.sig.ident, "on_raw_event is missing `event_type`")
            .to_compile_error()
            .into();
    };

    let cooldown_rule = cooldown_secs.map(|secs| {
        quote! {
            .rule({
                let __r: Box<dyn vkontakte::dispatch::rules::Rule<serde_json::Value>> =
                    Box::new(vkontakte::dispatch::rules::CooldownRule::per_peer(
                        std::time::Duration::from_secs(#secs)
                    ));
                __r
            })
        }
    });

    let register_name = syn::Ident::new(&format!("register_{func_name}"), func_name.span());

    quote! {
        #func

        #[allow(dead_code)]
        pub fn #register_name(bot: &mut vkontakte::framework::Bot) {
            bot.on()
                .raw_event(#event_type)
                #cooldown_rule
                .handle(|event, ctx| async move {
                    #func_name(event, ctx).await
                });
        }
    }
    .into()
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

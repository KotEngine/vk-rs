//! Router introspection — what got registered, and where.
//!
//! Answers the "is my handler even registered?" question without reading the
//! registration code. A snapshot is captured by `Bot::sync_router`, because the
//! labeler is drained into the router at that point.

use std::fmt::Write as _;

/// Which view a handler ended up under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteKind {
    Message,
    MessageEvent,
    RawEvent,
    RawValue,
}

impl RouteKind {
    pub fn view_name(self) -> &'static str {
        match self {
            Self::Message => "MessageView",
            Self::MessageEvent => "MessageEventView",
            Self::RawEvent => "RawEventView",
            Self::RawValue => "RawValueHandlers",
        }
    }
}

/// One registered handler.
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub kind: RouteKind,
    /// VK event type this route is scoped to — raw event routes only.
    pub event_type: Option<String>,
    /// Rules that must pass, as reported by the handler.
    pub rules: String,
}

impl RouteInfo {
    pub fn new(kind: RouteKind, rules: String) -> Self {
        Self {
            kind,
            event_type: None,
            rules,
        }
    }

    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }
}

/// Render routes as a tree, grouped by view.
///
/// ```text
/// === vkontakte Router ===
///
/// MessageView
///  ├─ [TextRule(ping)]
///  └─ [CommandRule(help)]
///
/// Blueprints mounted: admin, profile
/// Total handlers: 2
/// ```
pub fn format_routes(routes: &[RouteInfo], blueprints: &[String]) -> String {
    let mut out = String::from("=== vkontakte Router ===\n");

    let kinds = [
        RouteKind::Message,
        RouteKind::MessageEvent,
        RouteKind::RawEvent,
        RouteKind::RawValue,
    ];

    for kind in kinds {
        let group: Vec<&RouteInfo> = routes.iter().filter(|r| r.kind == kind).collect();
        if group.is_empty() {
            continue;
        }

        let _ = write!(out, "\n{}\n", kind.view_name());
        for (idx, route) in group.iter().enumerate() {
            let branch = if idx + 1 == group.len() {
                " └─"
            } else {
                " ├─"
            };
            match &route.event_type {
                Some(event_type) => {
                    let _ = writeln!(out, "{branch} {event_type} {}", route.rules);
                }
                None => {
                    let _ = writeln!(out, "{branch} {}", route.rules);
                }
            }
        }
    }

    if routes.is_empty() {
        out.push_str("\n(no handlers registered)\n");
    }

    if !blueprints.is_empty() {
        let _ = write!(out, "\nBlueprints mounted: {}\n", blueprints.join(", "));
    }
    let _ = write!(out, "Total handlers: {}\n", routes.len());

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_router_says_so() {
        let out = format_routes(&[], &[]);
        assert!(out.contains("(no handlers registered)"));
        assert!(out.contains("Total handlers: 0"));
    }

    #[test]
    fn routes_are_grouped_by_view() {
        let routes = vec![
            RouteInfo::new(RouteKind::Message, "[TextRule(ping)]".into()),
            RouteInfo::new(RouteKind::Message, "[CommandRule(help)]".into()),
            RouteInfo::new(RouteKind::MessageEvent, "[PayloadRule({})]".into()),
            RouteInfo::new(RouteKind::RawEvent, "[no rules]".into())
                .with_event_type("wall_post_new"),
        ];

        let out = format_routes(&routes, &["admin".into()]);

        assert!(out.contains("MessageView"));
        assert!(out.contains("MessageEventView"));
        assert!(out.contains("RawEventView"));
        assert!(out.contains("wall_post_new"));
        assert!(out.contains("Blueprints mounted: admin"));
        assert!(out.contains("Total handlers: 4"));
        // Last entry in a group gets the closing branch.
        assert!(out.contains(" └─ [CommandRule(help)]"));
        assert!(out.contains(" ├─ [TextRule(ping)]"));
    }

    #[test]
    fn views_without_routes_are_skipped() {
        let routes = vec![RouteInfo::new(RouteKind::Message, "[TextRule(x)]".into())];
        let out = format_routes(&routes, &[]);

        assert!(out.contains("MessageView"));
        assert!(!out.contains("RawEventView"));
        assert!(!out.contains("Blueprints mounted"));
    }
}

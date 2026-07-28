//! OAuth helpers for obtaining user tokens

use std::collections::HashMap;

/// Access rights a user token can carry.
///
/// The numeric value is the *bit position* as documented by VK — build a scope
/// mask with [`scope_mask`].
///
/// See <https://dev.vk.com/reference/access-rights>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum UserPermission {
    Notify = 0,
    Friends = 1,
    Photos = 2,
    Audio = 3,
    Video = 4,
    Stories = 6,
    Pages = 7,
    Menu = 8,
    WallMenu = 9,
    Status = 10,
    Notes = 11,
    Messages = 12,
    Wall = 13,
    Ads = 15,
    Offline = 16,
    Docs = 17,
    Groups = 18,
    Notifications = 19,
    Stats = 20,
    Email = 22,
    AdsWeb = 23,
    Leads = 24,
    Exchange = 26,
    Market = 27,
    PhoneNumber = 28,
}

impl UserPermission {
    /// Bit position of this permission.
    pub fn bit(self) -> u32 {
        self as u32
    }

    /// This permission as a standalone mask.
    pub fn mask(self) -> i64 {
        1i64 << self.bit()
    }

    /// Scope name accepted by the `scope=` query parameter.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Notify => "notify",
            Self::Friends => "friends",
            Self::Photos => "photos",
            Self::Audio => "audio",
            Self::Video => "video",
            Self::Stories => "stories",
            Self::Pages => "pages",
            Self::Menu => "menu",
            Self::WallMenu => "wallmenu",
            Self::Status => "status",
            Self::Notes => "notes",
            Self::Messages => "messages",
            Self::Wall => "wall",
            Self::Ads => "ads",
            Self::Offline => "offline",
            Self::Docs => "docs",
            Self::Groups => "groups",
            Self::Notifications => "notifications",
            Self::Stats => "stats",
            Self::Email => "email",
            Self::AdsWeb => "adsweb",
            Self::Leads => "leads",
            Self::Exchange => "exchange",
            Self::Market => "market",
            Self::PhoneNumber => "phone_number",
        }
    }
}

/// Permissions requested by default for a user token.
pub const DEFAULT_USER_PERMISSIONS: &[UserPermission] = &[
    UserPermission::Friends,
    UserPermission::Photos,
    UserPermission::Video,
    UserPermission::Stories,
    UserPermission::Pages,
    UserPermission::Status,
    UserPermission::Notes,
    UserPermission::Wall,
    UserPermission::Ads,
    UserPermission::Offline,
    UserPermission::Docs,
    UserPermission::Groups,
    UserPermission::Notifications,
    UserPermission::Stats,
    UserPermission::Email,
    UserPermission::AdsWeb,
    UserPermission::Exchange,
    UserPermission::Market,
];

/// Combine permissions into the integer bitmask VK expects in `scope=`.
pub fn scope_mask(permissions: &[UserPermission]) -> i64 {
    permissions.iter().fold(0, |acc, p| acc | p.mask())
}

/// Comma-separated scope string, the alternative to [`scope_mask`].
pub fn scope_names(permissions: &[UserPermission]) -> String {
    permissions
        .iter()
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// Build VK OAuth authorize URL (implicit flow with `token` response type)
pub fn build_implicit_auth_url(
    app_id: i64,
    redirect_uri: &str,
    scope: &[&str],
    state: Option<&str>,
) -> String {
    let scope_str = scope.join(",");
    let mut params = vec![
        ("client_id".to_string(), app_id.to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("display".to_string(), "page".to_string()),
        ("scope".to_string(), scope_str),
        ("response_type".to_string(), "token".to_string()),
        ("v".to_string(), crate::constants::VK_API_VERSION.to_string()),
    ];
    if let Some(s) = state {
        params.push(("state".to_string(), s.to_string()));
    }
    format_url(crate::constants::VK_OAUTH_URL, &params)
}

/// Build OAuth URL for authorization code flow
pub fn build_code_auth_url(
    app_id: i64,
    redirect_uri: &str,
    scope: &[&str],
    state: Option<&str>,
) -> String {
    let scope_str = scope.join(",");
    let mut params = vec![
        ("client_id".to_string(), app_id.to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("display".to_string(), "page".to_string()),
        ("scope".to_string(), scope_str),
        ("response_type".to_string(), "code".to_string()),
        ("v".to_string(), crate::constants::VK_API_VERSION.to_string()),
    ];
    if let Some(s) = state {
        params.push(("state".to_string(), s.to_string()));
    }
    format_url(crate::constants::VK_OAUTH_URL, &params)
}

/// Exchange authorization code for access token (server-side)
pub fn build_token_exchange_url(
    app_id: i64,
    app_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> String {
    let params = vec![
        ("client_id".to_string(), app_id.to_string()),
        ("client_secret".to_string(), app_secret.to_string()),
        ("redirect_uri".to_string(), redirect_uri.to_string()),
        ("code".to_string(), code.to_string()),
    ];
    format_url(crate::constants::VK_OAUTH_TOKEN_URL, &params)
}

fn format_url(base: &str, params: &[(String, String)]) -> String {
    let query: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}?{query}")
}

fn urlencoding_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// Parse access_token from redirect URL fragment or query
pub fn parse_token_from_redirect(url: &str) -> Option<HashMap<String, String>> {
    let part = url.split('#').nth(1).or_else(|| url.split('?').nth(1))?;
    let mut map = HashMap::new();
    for pair in part.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    if map.contains_key("access_token") {
        Some(map)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_mask_matches_vk_bit_positions() {
        assert_eq!(scope_mask(&[UserPermission::Notify]), 1);
        assert_eq!(scope_mask(&[UserPermission::Friends]), 2);
        assert_eq!(scope_mask(&[UserPermission::Messages]), 4096);
        assert_eq!(scope_mask(&[UserPermission::Offline]), 65536);
        assert_eq!(
            scope_mask(&[UserPermission::Friends, UserPermission::Photos]),
            2 + 4
        );
        assert_eq!(scope_mask(&[]), 0);
    }

    #[test]
    fn repeated_permission_does_not_double_count() {
        assert_eq!(
            scope_mask(&[UserPermission::Wall, UserPermission::Wall]),
            UserPermission::Wall.mask()
        );
    }

    #[test]
    fn scope_names_are_comma_separated() {
        assert_eq!(
            scope_names(&[UserPermission::Messages, UserPermission::Offline]),
            "messages,offline"
        );
    }

    #[test]
    fn default_permissions_have_expected_mask() {
        // Sum of 2^bit over DEFAULT_USER_PERMISSIONS.
        assert_eq!(scope_mask(DEFAULT_USER_PERMISSIONS), 215_985_366);
    }

    #[test]
    fn implicit_url_contains_client_id() {
        let url = build_implicit_auth_url(123, "https://example.com/cb", &["messages"], None);
        assert!(url.contains("client_id=123"));
        assert!(url.contains("response_type=token"));
    }
}

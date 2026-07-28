//! Finite state machine types

use std::collections::HashMap;

use serde_json::Value;

use crate::dispatch::dispenser::StateDispenser;
use crate::exception::VkResult;

/// Peer state for FSM
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StatePeer {
    pub peer_id: i64,
    pub state: String,
    pub payload: HashMap<String, Value>,
}

impl StatePeer {
    pub fn new(peer_id: i64, state: impl Into<String>) -> Self {
        Self {
            peer_id,
            state: state.into(),
            payload: HashMap::new(),
        }
    }

    pub fn with_payload(mut self, payload: HashMap<String, Value>) -> Self {
        self.payload = payload;
        self
    }

    pub fn set_payload(&mut self, key: impl Into<String>, value: Value) {
        self.payload.insert(key.into(), value);
    }

    pub fn get_payload(&self, key: &str) -> Option<&Value> {
        self.payload.get(key)
    }

    pub fn group_name(&self) -> &str {
        crate::dispatch::state_context::state_group_name(&self.state)
    }
}

/// Set peer state via any dispenser
pub async fn set_peer_state(
    dispenser: &dyn StateDispenser,
    peer_id: i64,
    state: impl Into<String>,
) -> VkResult<()> {
    dispenser.set(StatePeer::new(peer_id, state)).await
}

/// Set peer state with payload map
pub async fn set_peer_state_with_payload(
    dispenser: &dyn StateDispenser,
    peer_id: i64,
    state: impl Into<String>,
    payload: HashMap<String, Value>,
) -> VkResult<()> {
    dispenser
        .set(StatePeer::new(peer_id, state).with_payload(payload))
        .await
}

/// Delete peer state
pub async fn delete_peer_state(dispenser: &dyn StateDispenser, peer_id: i64) -> VkResult<bool> {
    dispenser.delete(peer_id).await
}

/// Trait for typed state groups (`MenuState::Start.as_str()` → `MenuState:start`)
pub trait StateGroupValue {
    fn group_name() -> &'static str;
    fn as_str(&self) -> String;
}

/// Declare a named FSM state group
///
/// ```ignore
/// state_group! {
///     pub enum MenuState {
///         Start = "start",
///         Info = "info",
///     }
/// }
/// ```
#[macro_export]
macro_rules! state_group {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident = $value:literal),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant),*
        }

        impl $name {
            pub fn group_name() -> &'static str {
                stringify!($name)
            }

            pub fn as_str(&self) -> String {
                match self {
                    $(Self::$variant => $crate::tools::fsm::make_state_repr(stringify!($name), $value)),*
                }
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.as_str()
            }
        }

        impl $crate::tools::fsm::StateGroupValue for $name {
            fn group_name() -> &'static str {
                stringify!($name)
            }

            fn as_str(&self) -> String {
                match self {
                    $(Self::$variant => $crate::tools::fsm::make_state_repr(stringify!($name), $value)),*
                }
            }
        }
    };
}

/// Build `Group:value` state repr (re-exported for macro expansion)
pub fn make_state_repr(group: &str, value: &str) -> String {
    crate::dispatch::state_context::make_state_repr(group, value)
}

/// Named state group for StateGroupRule
#[derive(Debug, Clone)]
pub struct StateGroup {
    pub name: String,
    pub states: Vec<String>,
}

impl StateGroup {
    pub fn new(name: impl Into<String>, states: Vec<String>) -> Self {
        Self {
            name: name.into(),
            states,
        }
    }

    pub fn contains(&self, state: &str) -> bool {
        self.states.iter().any(|s| s == state)
    }
}

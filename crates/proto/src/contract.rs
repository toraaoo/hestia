//! Conventions for the typed wire contracts. A call `Contract` names its channel
//! once and pairs it with the `Params`/`Result` payload shapes; both sides of the
//! socket marshal through one definition and cannot drift — a disagreement is a
//! compile error. Serde derive is the marshalling layer (replacing the C++
//! `kFields` codec).

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// Binds a channel string to its request/response payload types.
pub trait Contract {
    const CHANNEL: &'static str;
    type Params: Serialize + DeserializeOwned;
    type Result: Serialize + DeserializeOwned;
}

/// A topic string for an unsolicited daemon→client event; the implementing type
/// is its own payload.
pub trait Topic {
    const TOPIC: &'static str;
}

/// An empty payload: serializes to `{}` and decodes from it, matching the C++
/// `proto::Empty`.
#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq, Eq)]
pub struct Empty {}

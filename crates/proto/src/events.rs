use serde::{Deserialize, Serialize};

use crate::contract::Contract;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct EventsSubscribeParams {
    /// Empty subscribes to every event.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct EventsSubscribeResult {
    pub subscribed: bool,
}

pub struct EventsSubscribe;
impl Contract for EventsSubscribe {
    const CHANNEL: &'static str = "events.subscribe";
    type Params = EventsSubscribeParams;
    type Result = EventsSubscribeResult;
}

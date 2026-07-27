//! Cancelling a long-running operation.
//!
//! Every job — a JDK download, a server create, an instance launch, a backup, a
//! content install — is already identified by the client-supplied job id its
//! progress and terminal events carry. Cancellation reuses exactly that id, so
//! there is **one** channel rather than a `cancel` verb per domain: a front-end
//! cancels the run it started, whatever kind of run it was.
//!
//! A daemon job outlives the client that asked for it (see the "workloads
//! outlive the daemon" note in the architecture doc), so a disconnect must never
//! cancel anything. Cancelling is therefore an explicit act, like stopping a
//! workload — this channel is the only way it happens.

use serde::{Deserialize, Serialize};

use crate::contract::Contract;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct JobCancelParams {
    /// The job id its progress and terminal events carry.
    pub id: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export, optional_fields))]
#[serde(default, rename_all = "camelCase")]
pub struct JobCancelResult {
    /// False when no job of that id is running — it already finished, or never
    /// existed. Not an error: asking to cancel something already over is a
    /// normal race, not a mistake.
    pub cancelled: bool,
}

pub struct JobCancel;
impl Contract for JobCancel {
    const CHANNEL: &'static str = "job.cancel";
    type Params = JobCancelParams;
    type Result = JobCancelResult;
}

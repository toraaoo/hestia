//! Global content profiles: data-home-level *project reference* lists, applied
//! into an instance's pool as ordinary tagged content. A profile stores
//! references (`source` + `project_id`), never jars — jars are version- and
//! loader-specific, so each apply resolves per instance.

use serde::{Deserialize, Serialize};

use crate::content::ContentJobResult;
use crate::contract::{Contract, Empty};

/// One project reference of a global profile. `slug` is carried for display
/// and matching; `source` + `project_id` identify the project.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProfileEntry {
    pub source: String,
    pub project_id: String,
    pub slug: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct GlobalProfile {
    pub name: String,
    pub entries: Vec<ProfileEntry>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProfileListResult {
    pub profiles: Vec<GlobalProfile>,
}

pub struct ProfileList;
impl Contract for ProfileList {
    const CHANNEL: &'static str = "profile.list";
    type Params = Empty;
    type Result = ProfileListResult;
}

/// Names one global profile (create / remove).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProfileRef {
    pub name: String,
}

pub struct ProfileCreate;
impl Contract for ProfileCreate {
    const CHANNEL: &'static str = "profile.create";
    type Params = ProfileRef;
    type Result = GlobalProfile;
}

pub struct ProfileRemove;
impl Contract for ProfileRemove {
    const CHANNEL: &'static str = "profile.remove";
    type Params = ProfileRef;
    type Result = Empty;
}

/// `add`/`remove` are project references (slug or id); adds are resolved
/// through the content registry on `source` (empty = the default source).
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct ProfileEditParams {
    pub name: String,
    pub source: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

pub struct ProfileEdit;
impl Contract for ProfileEdit {
    const CHANNEL: &'static str = "profile.edit";
    type Params = ProfileEditParams;
    type Result = GlobalProfile;
}

/// Apply a global profile into an instance's pool — a job publishing the
/// `content.*` topics. Entries already in the pool (any origin) are skipped;
/// an entry with no compatible version is reported as a failure and the batch
/// continues. Applying never removes de-listed content.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub struct InstanceProfileApplyParams {
    pub instance: String,
    pub profile: String,
    /// Client-supplied job id; empty asks the daemon to allocate one.
    pub id: String,
}

pub struct InstanceProfileApply;
impl Contract for InstanceProfileApply {
    const CHANNEL: &'static str = "instance.profile.apply";
    type Params = InstanceProfileApplyParams;
    type Result = ContentJobResult;
}

//! The canonical daemon error — a structured, exhaustive value carried whole
//! across the socket. Nobody authors prose at a call site: the English text
//! (`Display`) and the coarse `code` are projections of the variant, and a
//! front-end renders its own localized string from the tag + typed fields.
//!
//! Semantic variants translate fully with their fields; operational variants
//! (`Io`, `Upstream`, `DownloadFailed`, `RconFailed`, `Internal`) carry an
//! unbounded English `detail` — a per-path filesystem message is not localizable
//! and is shown as secondary text under a translated headline.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A launcher entry that resolves by reference.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Server,
    Instance,
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EntryKind::Server => "server",
            EntryKind::Instance => "instance",
        })
    }
}

/// A uniquely-named thing that can already exist.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Nameable {
    Server,
    Instance,
    Profile,
    GlobalProfile,
}

impl fmt::Display for Nameable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Nameable::Server => "server",
            Nameable::Instance => "instance",
            Nameable::Profile => "profile",
            Nameable::GlobalProfile => "global profile",
        })
    }
}

/// Which profile namespace a lookup missed.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileScope {
    Instance,
    Global,
}

impl fmt::Display for ProfileScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ProfileScope::Instance => "instance",
            ProfileScope::Global => "global",
        })
    }
}

/// A required-or-invalid input, named so a front-end can label it.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Name,
    Project,
    Version,
    Item,
    Backup,
    Command,
    Program,
    Url,
    Path,
    Flavor,
    World,
    Memory,
    JvmArgs,
    Port,
    Players,
    BackupInterval,
    BackupRetention,
    JavaVersion,
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Field::Name => "a name",
            Field::Project => "a project",
            Field::Version => "a version",
            Field::Item => "an item",
            Field::Backup => "a backup",
            Field::Command => "a command",
            Field::Program => "a program",
            Field::Url => "a download url",
            Field::Path => "a file path",
            Field::Flavor => "a flavor",
            Field::World => "a world",
            Field::Memory => "memory",
            Field::JvmArgs => "jvm arguments",
            Field::Port => "a port",
            Field::Players => "players",
            Field::BackupInterval => "backup-interval",
            Field::BackupRetention => "backup-retention",
            Field::JavaVersion => "a java version",
        })
    }
}

/// A closed reason an otherwise-present value was rejected.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    MemoryFormat,
    JvmArgsPrefix,
    PortNumber,
    PortRange,
    WholeNumber,
    IntervalFormat,
    IntervalTooShort,
    RetentionPositive,
    MinPlayers,
    MinBackups,
    JavaMajor,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reason::MemoryFormat => "memory must look like 4G or 2048M",
            Reason::JvmArgsPrefix => "jvm arguments must start with '-'",
            Reason::PortNumber => "port must be a number",
            Reason::PortRange => "port is out of range",
            Reason::WholeNumber => "enter a whole number",
            Reason::IntervalFormat => "backup-interval must look like 30m, 6h, or 1d",
            Reason::IntervalTooShort => "backup-interval must be at least 5m",
            Reason::RetentionPositive => "backup-retention must be a positive integer",
            Reason::MinPlayers => "at least one player is required",
            Reason::MinBackups => "keep at least one backup",
            Reason::JavaMajor => "not a valid java major version",
        })
    }
}

/// A domain rule that forbids an otherwise well-formed operation.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Unsupported {
    ServerContentKinds,
    VanillaNoMods,
    WorldsForDatapacksOnly,
    DatapacksPerWorld,
    ModpackNotSingleFile,
}

impl fmt::Display for Unsupported {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Unsupported::ServerContentKinds => "a server takes mods and datapacks only",
            Unsupported::VanillaNoMods => "a vanilla game cannot load mods",
            Unsupported::WorldsForDatapacksOnly => "worlds apply to datapacks only",
            Unsupported::DatapacksPerWorld => "only datapacks are installed per world",
            Unsupported::ModpackNotSingleFile => {
                "modpack content cannot be installed as a single file"
            }
        })
    }
}

/// An upstream service the daemon depends on.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Service {
    Adoptium,
    Mojang,
    Fabric,
    Modrinth,
    Microsoft,
    Xbox,
}

impl fmt::Display for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Service::Adoptium => "Adoptium",
            Service::Mojang => "Mojang",
            Service::Fabric => "Fabric",
            Service::Modrinth => "Modrinth",
            Service::Microsoft => "Microsoft",
            Service::Xbox => "Xbox",
        })
    }
}

/// The filesystem action an `Io` failure was performing.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IoOp {
    Create,
    Read,
    Write,
    Remove,
    Move,
    Copy,
    Open,
    Link,
    Unlink,
    Extract,
}

impl fmt::Display for IoOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IoOp::Create => "create",
            IoOp::Read => "read",
            IoOp::Write => "write",
            IoOp::Remove => "remove",
            IoOp::Move => "move",
            IoOp::Copy => "copy",
            IoOp::Open => "open",
            IoOp::Link => "link",
            IoOp::Unlink => "unlink",
            IoOp::Extract => "extract",
        })
    }
}

/// What there was nothing to do.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    Install,
    Modify,
    BackUp,
}

impl fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Task::Install => "install",
            Task::Modify => "change",
            Task::BackUp => "back up",
        })
    }
}

/// Why a path was rejected as a sync target.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    CopiedTarget,
    NotFolderTarget,
    ManagedDir,
    UnsafePath,
}

impl fmt::Display for SyncReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SyncReason::CopiedTarget => "share it as a folder instead",
            SyncReason::NotFolderTarget => "it is not a folder sync target",
            SyncReason::ManagedDir => "it is a launcher-managed directory",
            SyncReason::UnsafePath => "it is not a safe relative path",
        })
    }
}

/// The one daemon error type — every failure the socket surfaces. The `kind`
/// tag is the wire discriminant; front-ends switch on it exhaustively.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ErrorInfo {
    // --- validation ---
    FieldRequired { field: Field },
    FieldsRequired { fields: Vec<Field> },
    InvalidValue { field: Field, reason: Reason },
    MutuallyExclusive { options: Vec<String> },
    NothingToDo { what: Task },
    EulaRequired,
    Busy { detail: String },
    ReservedName { name: String },
    UnsupportedOperation { reason: Unsupported },
    InvalidTexture { detail: String },

    // --- not found ---
    EntryNotFound { entry: EntryKind, reference: String },
    ProcessNotFound { id: String },
    BackupNotFound { reference: String },
    ContentNotFound { reference: String },
    ProfileNotFound { scope: ProfileScope, name: String },
    SkinNotFound { key: String },
    WorldNotFound { world: String },
    AccountNotFound { reference: String },
    VersionNotFound { reference: String },
    ConfigKeyUnknown { key: String },
    ConfigKeyUnset { key: String },
    ConfigTypeMismatch { detail: String },
    ConfigRejected { key: String, detail: String },

    // --- conflict ---
    AlreadyExists { entry: Nameable, name: String },
    PortUnavailable { port: u16 },

    // --- state ---
    EntryRunning { entry: EntryKind, name: String },
    NotRunning { entry: EntryKind, name: String },
    Provisioning { name: String },
    UpdateInProgress { name: String },
    ContentInProgress { name: String },
    BackupInProgress { name: String },
    NoConsole { name: String },
    NoGamePort { name: String },
    ProfileAlreadyCaptured { name: String },
    ProfileNotCaptured { name: String },

    // --- auth ---
    SignInRequired,
    SessionExpired { reference: String },
    LoginDeclined,
    LoginTimedOut,

    // --- content / modpack ---
    NotAModpack { reference: String },
    ModpackInvalid { detail: String },

    // --- sync ---
    SyncTargetInvalid { path: String, reason: SyncReason },
    SyncLinkConflict { path: String },

    // --- protocol ---
    UnknownChannel { channel: String },
    MalformedRequest { detail: String },

    // --- operational (unbounded English `detail`) ---
    Io { operation: IoOp, detail: String },
    Upstream { service: Service, detail: String },
    DownloadFailed { detail: String },
    RconFailed { detail: String },
    Internal { detail: String },
}

impl ErrorInfo {
    /// The coarse `ipc::errors` category this variant answers with.
    pub fn code(&self) -> &'static str {
        use ErrorInfo::*;
        match self {
            EntryNotFound { .. }
            | ProcessNotFound { .. }
            | BackupNotFound { .. }
            | ContentNotFound { .. }
            | ProfileNotFound { .. }
            | SkinNotFound { .. }
            | WorldNotFound { .. }
            | AccountNotFound { .. }
            | VersionNotFound { .. }
            | ConfigKeyUnknown { .. }
            | ConfigKeyUnset { .. } => "not_found",
            SignInRequired | SessionExpired { .. } | LoginDeclined | LoginTimedOut => {
                "unauthorized"
            }
            UnknownChannel { .. } => "unknown_channel",
            Io { .. }
            | Upstream { .. }
            | DownloadFailed { .. }
            | RconFailed { .. }
            | Internal { .. } => "handler_error",
            _ => "bad_request",
        }
    }
}

impl fmt::Display for ErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ErrorInfo::*;
        match self {
            FieldRequired { field } => write!(f, "{field} is required"),
            FieldsRequired { fields } => {
                let joined = fields
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" and ");
                write!(f, "{joined} are required")
            }
            InvalidValue { reason, .. } => write!(f, "{reason}"),
            MutuallyExclusive { options } => {
                write!(f, "choose exactly one of: {}", options.join(", "))
            }
            NothingToDo { what } => write!(f, "nothing to {what}"),
            EulaRequired => write!(f, "accept the EULA to create a server"),
            Busy { detail } => write!(f, "{detail}"),
            ReservedName { name } => write!(f, "'{name}' is a reserved name"),
            UnsupportedOperation { reason } => write!(f, "{reason}"),
            InvalidTexture { detail } => write!(f, "{detail}"),
            EntryNotFound { entry, reference } => write!(f, "no {entry} matches '{reference}'"),
            ProcessNotFound { id } => write!(f, "no process '{id}'"),
            BackupNotFound { reference } => write!(f, "no backup matches '{reference}'"),
            ContentNotFound { reference } => {
                write!(f, "no installed content matches '{reference}'")
            }
            ProfileNotFound { scope, name } => write!(f, "no {scope} profile named '{name}'"),
            SkinNotFound { key } => write!(f, "no skin matches '{key}'"),
            WorldNotFound { world } => write!(f, "no world '{world}' in this instance"),
            AccountNotFound { reference } => write!(f, "no account matches '{reference}'"),
            VersionNotFound { reference } => write!(f, "no version matches '{reference}'"),
            ConfigKeyUnknown { key } => write!(f, "unknown config key '{key}'"),
            ConfigKeyUnset { key } => write!(f, "'{key}' is not set"),
            ConfigTypeMismatch { detail } => write!(f, "{detail}"),
            ConfigRejected { key, detail } => write!(f, "invalid value for {key}: {detail}"),
            AlreadyExists { entry, name } => write!(f, "a {entry} named '{name}' already exists"),
            PortUnavailable { port } => write!(f, "port {port} is unavailable"),
            EntryRunning { name, .. } => write!(f, "{name} is running — stop it first"),
            NotRunning { name, .. } => write!(f, "{name} is not running"),
            Provisioning { name } => write!(f, "{name} is still being set up"),
            UpdateInProgress { name } => write!(f, "{name} is being updated"),
            ContentInProgress { name } => write!(f, "{name} has a content change in progress"),
            BackupInProgress { name } => {
                write!(f, "{name} has a backup or restore in progress")
            }
            NoConsole { name } => write!(f, "{name} has no console yet — restart it"),
            NoGamePort { name } => write!(f, "{name} has no game port allocated"),
            ProfileAlreadyCaptured { name } => {
                write!(f, "profile '{name}' already captured its settings")
            }
            ProfileNotCaptured { name } => write!(f, "profile '{name}' has no captured settings"),
            SignInRequired => write!(f, "sign in with a Microsoft account first"),
            SessionExpired { reference } => {
                write!(f, "your sign-in for '{reference}' expired — sign in again")
            }
            LoginDeclined => write!(f, "the sign-in was declined"),
            LoginTimedOut => write!(f, "the sign-in timed out — try again"),
            NotAModpack { reference } => write!(f, "'{reference}' is not a modpack"),
            ModpackInvalid { detail } => write!(f, "this modpack could not be read: {detail}"),
            SyncTargetInvalid { path, reason } => {
                write!(f, "'{path}' cannot be a sync target: {reason}")
            }
            SyncLinkConflict { path } => {
                write!(f, "'{path}' already has contents — adopt it first")
            }
            UnknownChannel { channel } => write!(f, "unknown channel: {channel}"),
            MalformedRequest { detail } => write!(f, "malformed request: {detail}"),
            Io { operation, detail } => write!(f, "could not {operation}: {detail}"),
            Upstream { service, detail } => write!(f, "{service} request failed: {detail}"),
            DownloadFailed { detail } => write!(f, "download failed: {detail}"),
            RconFailed { detail } => write!(f, "server console command failed: {detail}"),
            Internal { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for ErrorInfo {}

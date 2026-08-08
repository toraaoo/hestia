//! What each daemon failure becomes over HTTP, and what a remote caller is not
//! allowed to learn on the way. The match is exhaustive on purpose: a new
//! `ErrorInfo` variant fails the build until somebody decides what it means off
//! this machine. `code` is the nine-value closed set of `API_ERROR_CODES`.

use axum::http::StatusCode;
use proto::error::ErrorInfo;

pub const BAD_REQUEST: &str = "BAD_REQUEST";
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
pub const FORBIDDEN: &str = "FORBIDDEN";
pub const NOT_FOUND: &str = "NOT_FOUND";
pub const CONFLICT: &str = "CONFLICT";
pub const VALIDATION_FAILED: &str = "VALIDATION_FAILED";
pub const TOO_MANY_REQUESTS: &str = "TOO_MANY_REQUESTS";
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";

/// What a remote caller is told instead of a path.
const REDACTED: &str = "redacted — see the daemon log on the node";

pub fn of(info: &ErrorInfo) -> (StatusCode, &'static str) {
    use ErrorInfo::*;
    match info {
        FieldRequired { .. }
        | FieldsRequired { .. }
        | InvalidValue { .. }
        | MutuallyExclusive { .. }
        | UnsupportedOperation { .. }
        | ContentKindRejected { .. }
        | ContentKindMismatch { .. }
        | InvalidTexture { .. }
        | QuickPlayUnsupported { .. }
        | UnsupportedContentUrl { .. }
        | ConfigTypeMismatch { .. }
        | ConfigRejected { .. }
        | NotAModpack { .. }
        | ModpackInvalid { .. }
        | ModpackLoaderUnsupported { .. }
        | ModpackEntryMismatch { .. }
        | ArchiveUnrecognised { .. }
        | ArchiveInvalid { .. }
        | ArchiveUnsupported { .. }
        | SyncTargetInvalid { .. } => (StatusCode::UNPROCESSABLE_ENTITY, VALIDATION_FAILED),
        MalformedRequest { .. } => (StatusCode::BAD_REQUEST, BAD_REQUEST),
        IncompatibleVersion { .. } => (StatusCode::BAD_REQUEST, BAD_REQUEST),
        EulaRequired
        | Busy { .. }
        | ReservedName { .. }
        | NothingToDo { .. }
        | AlreadyExists { .. }
        | PortUnavailable { .. }
        | EntryRunning { .. }
        | MultiSessionDisabled { .. }
        | NotRunning { .. }
        | Provisioning { .. }
        | UpdateInProgress { .. }
        | ContentInProgress { .. }
        | BackupInProgress { .. }
        | NoConsole { .. }
        | NoGamePort { .. }
        | ProfileAlreadyCaptured { .. }
        | ProfileNotCaptured { .. }
        | SyncLinkConflict { .. } => (StatusCode::CONFLICT, CONFLICT),
        EntryNotFound { .. }
        | ProcessNotFound { .. }
        | BackupNotFound { .. }
        | ContentNotFound { .. }
        | ProfileNotFound { .. }
        | SkinNotFound { .. }
        | WorldNotFound { .. }
        | ServerListEntryNotFound { .. }
        | AccountNotFound { .. }
        | VersionNotFound { .. }
        | ConfigKeyUnknown { .. }
        | ConfigKeyUnset { .. }
        | ModpackNotInstalled { .. }
        | RemoteKeyNotFound { .. }
        // Unreachable unless a route names a channel the daemon does not
        // serve, which the caller cannot tell from a wrong URL.
        | UnknownChannel { .. } => (StatusCode::NOT_FOUND, NOT_FOUND),
        RemoteKeyRejected => (StatusCode::UNAUTHORIZED, UNAUTHORIZED),
        SessionExpired { .. } => (StatusCode::UNAUTHORIZED, UNAUTHORIZED),
        TooManyAttempts { .. } => (StatusCode::TOO_MANY_REQUESTS, TOO_MANY_REQUESTS),
        RemoteScopeRequired { .. } => (StatusCode::FORBIDDEN, FORBIDDEN),
        // Unreachable over HTTP; present so the match stays exhaustive.
        SignInRequired | LoginDeclined | LoginTimedOut | ElevationRequired { .. } => {
            (StatusCode::FORBIDDEN, FORBIDDEN)
        }
        // 424: the node is missing a dependency, not the request.
        MissingRequirement { .. } => (StatusCode::FAILED_DEPENDENCY, BAD_REQUEST),
        ContentDownloadBlocked { .. } => (
            StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
            SERVICE_UNAVAILABLE,
        ),
        ContentSourceUnavailable { .. } | Offline { .. } | OfflineMode => {
            (StatusCode::SERVICE_UNAVAILABLE, SERVICE_UNAVAILABLE)
        }
        Upstream { .. } => (StatusCode::BAD_GATEWAY, SERVICE_UNAVAILABLE),
        Io { .. } | DownloadFailed { .. } | RconFailed { .. } | Internal { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR)
        }
    }
}

/// Strip what a remote caller must not see: these variants carry an `io::Error`
/// string, which names the absolute path it failed on. The variant and its
/// status survive — only the prose goes, and the daemon's log still has it.
pub fn sanitize(info: ErrorInfo) -> ErrorInfo {
    match info {
        ErrorInfo::Io { operation, .. } => ErrorInfo::Io {
            operation,
            detail: REDACTED.to_string(),
        },
        ErrorInfo::Internal { .. } => ErrorInfo::Internal {
            detail: REDACTED.to_string(),
        },
        ErrorInfo::ArchiveInvalid { format, .. } => ErrorInfo::ArchiveInvalid {
            format,
            detail: REDACTED.to_string(),
        },
        ErrorInfo::SyncTargetInvalid { reason, .. } => ErrorInfo::SyncTargetInvalid {
            path: REDACTED.to_string(),
            reason,
        },
        ErrorInfo::SyncLinkConflict { .. } => ErrorInfo::SyncLinkConflict {
            path: REDACTED.to_string(),
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proto::error::{IoOp, SyncReason};

    /// The nine `API_ERROR_CODES` `hestia-web`'s contract defines, and nothing
    /// else: a tenth invented here is a code no generic integrator handles.
    const CODES: &[&str] = &[
        BAD_REQUEST,
        UNAUTHORIZED,
        FORBIDDEN,
        NOT_FOUND,
        CONFLICT,
        VALIDATION_FAILED,
        TOO_MANY_REQUESTS,
        INTERNAL_ERROR,
        SERVICE_UNAVAILABLE,
    ];

    fn every_variant() -> Vec<ErrorInfo> {
        use proto::content::ContentKind;
        use proto::error::{EntryKind, Field, Nameable, ProfileScope, Reason, Service, Task};
        use proto::error::{IoOp as Op, Unsupported};
        use proto::remote::Scope;
        vec![
            ErrorInfo::FieldRequired { field: Field::Name },
            ErrorInfo::FieldsRequired { fields: vec![] },
            ErrorInfo::InvalidValue {
                field: Field::Port,
                reason: Reason::PortNumber,
            },
            ErrorInfo::MutuallyExclusive { options: vec![] },
            ErrorInfo::NothingToDo {
                what: Task::Install,
            },
            ErrorInfo::EulaRequired,
            ErrorInfo::Busy {
                detail: String::new(),
            },
            ErrorInfo::ReservedName {
                name: String::new(),
            },
            ErrorInfo::UnsupportedOperation {
                reason: Unsupported::DatapacksPerWorld,
            },
            ErrorInfo::ContentKindRejected {
                entry: EntryKind::Server,
                flavor: String::new(),
                requested: ContentKind::Mod,
                accepts: vec![],
            },
            ErrorInfo::MissingRequirement {
                flavor: String::new(),
                name: String::new(),
                url: String::new(),
            },
            ErrorInfo::InvalidTexture {
                detail: String::new(),
            },
            ErrorInfo::QuickPlayUnsupported {
                version: String::new(),
            },
            ErrorInfo::EntryNotFound {
                entry: EntryKind::Server,
                reference: String::new(),
            },
            ErrorInfo::ProcessNotFound { id: String::new() },
            ErrorInfo::BackupNotFound {
                reference: String::new(),
            },
            ErrorInfo::ContentNotFound {
                reference: String::new(),
            },
            ErrorInfo::ProfileNotFound {
                scope: ProfileScope::Global,
                name: String::new(),
            },
            ErrorInfo::SkinNotFound { key: String::new() },
            ErrorInfo::WorldNotFound {
                world: String::new(),
            },
            ErrorInfo::ServerListEntryNotFound {
                reference: String::new(),
            },
            ErrorInfo::AccountNotFound {
                reference: String::new(),
            },
            ErrorInfo::VersionNotFound {
                reference: String::new(),
            },
            ErrorInfo::ConfigKeyUnknown { key: String::new() },
            ErrorInfo::ConfigKeyUnset { key: String::new() },
            ErrorInfo::ConfigTypeMismatch {
                detail: String::new(),
            },
            ErrorInfo::ConfigRejected {
                key: String::new(),
                detail: String::new(),
            },
            ErrorInfo::RemoteKeyNotFound {
                reference: String::new(),
            },
            ErrorInfo::AlreadyExists {
                entry: Nameable::Server,
                name: String::new(),
            },
            ErrorInfo::PortUnavailable { port: 0 },
            ErrorInfo::EntryRunning {
                entry: EntryKind::Server,
                name: String::new(),
            },
            ErrorInfo::MultiSessionDisabled {
                name: String::new(),
            },
            ErrorInfo::NotRunning {
                entry: EntryKind::Server,
                name: String::new(),
            },
            ErrorInfo::Provisioning {
                name: String::new(),
            },
            ErrorInfo::UpdateInProgress {
                name: String::new(),
            },
            ErrorInfo::ContentInProgress {
                name: String::new(),
            },
            ErrorInfo::BackupInProgress {
                name: String::new(),
            },
            ErrorInfo::NoConsole {
                name: String::new(),
            },
            ErrorInfo::NoGamePort {
                name: String::new(),
            },
            ErrorInfo::ProfileAlreadyCaptured {
                name: String::new(),
            },
            ErrorInfo::ProfileNotCaptured {
                name: String::new(),
            },
            ErrorInfo::SignInRequired,
            ErrorInfo::RemoteKeyRejected,
            ErrorInfo::RemoteScopeRequired {
                scope: Scope::ServerRead,
            },
            ErrorInfo::TooManyAttempts {
                retry_after_seconds: 30,
            },
            ErrorInfo::SessionExpired {
                reference: String::new(),
            },
            ErrorInfo::LoginDeclined,
            ErrorInfo::LoginTimedOut,
            ErrorInfo::NotAModpack {
                reference: String::new(),
            },
            ErrorInfo::ModpackInvalid {
                detail: String::new(),
            },
            ErrorInfo::ModpackLoaderUnsupported {
                loader: String::new(),
            },
            ErrorInfo::ModpackEntryMismatch {
                entry: EntryKind::Server,
                flavor: String::new(),
                game_version: String::new(),
                pack_flavor: String::new(),
                pack_game_version: String::new(),
            },
            ErrorInfo::ModpackNotInstalled {
                entry: EntryKind::Server,
                name: String::new(),
            },
            ErrorInfo::UnsupportedContentUrl { url: String::new() },
            ErrorInfo::ArchiveUnrecognised {
                filename: String::new(),
            },
            ErrorInfo::ArchiveInvalid {
                format: String::new(),
                detail: String::new(),
            },
            ErrorInfo::ArchiveUnsupported {
                format: String::new(),
                component: String::new(),
            },
            ErrorInfo::ContentKindMismatch {
                title: String::new(),
                actual: ContentKind::Mod,
                expected: ContentKind::Plugin,
            },
            ErrorInfo::ContentSourceUnavailable {
                source: String::new(),
            },
            ErrorInfo::ContentDownloadBlocked {
                title: String::new(),
                source: String::new(),
            },
            ErrorInfo::SyncTargetInvalid {
                path: String::new(),
                reason: SyncReason::ManagedDir,
            },
            ErrorInfo::SyncLinkConflict {
                path: String::new(),
            },
            ErrorInfo::UnknownChannel {
                channel: String::new(),
            },
            ErrorInfo::MalformedRequest {
                detail: String::new(),
            },
            ErrorInfo::IncompatibleVersion { got: 0, want: 1 },
            ErrorInfo::ElevationRequired {
                command: String::new(),
            },
            ErrorInfo::Io {
                operation: Op::Read,
                detail: String::new(),
            },
            ErrorInfo::Offline {
                service: Some(Service::Modrinth),
            },
            ErrorInfo::OfflineMode,
            ErrorInfo::Upstream {
                service: Service::Modrinth,
                detail: String::new(),
            },
            ErrorInfo::DownloadFailed {
                detail: String::new(),
            },
            ErrorInfo::RconFailed {
                detail: String::new(),
            },
            ErrorInfo::Internal {
                detail: String::new(),
            },
        ]
    }

    #[test]
    fn every_variant_answers_with_a_code_a_generic_client_knows() {
        for info in every_variant() {
            let (status, code) = of(&info);
            assert!(CODES.contains(&code), "{info:?} invented the code {code}");
            assert!(
                status.is_client_error() || status.is_server_error(),
                "{info:?} answered {status}, which is not a failure"
            );
        }
    }

    #[test]
    fn a_failure_never_answers_with_a_success_status() {
        assert!(every_variant().iter().all(|i| of(i).0.as_u16() >= 400));
    }

    #[test]
    fn an_io_failure_stops_naming_the_path_it_failed_on() {
        let leaked = ErrorInfo::Io {
            operation: IoOp::Read,
            detail: "No such file or directory (os error 2): /home/op/.hestia/servers/smp".into(),
        };
        let ErrorInfo::Io { detail, operation } = sanitize(leaked) else {
            panic!("the variant must survive sanitisation");
        };
        assert_eq!(operation, IoOp::Read);
        assert!(!detail.contains("/home/op"));
        assert_eq!(detail, REDACTED);
    }

    #[test]
    fn every_variant_carrying_a_path_is_sanitised() {
        let carriers = [
            ErrorInfo::Io {
                operation: IoOp::Read,
                detail: "/home/op/secret".into(),
            },
            ErrorInfo::Internal {
                detail: "/home/op/secret".into(),
            },
            ErrorInfo::ArchiveInvalid {
                format: "mrpack".into(),
                detail: "/home/op/secret".into(),
            },
            ErrorInfo::SyncTargetInvalid {
                path: "/home/op/secret".into(),
                reason: SyncReason::ManagedDir,
            },
            ErrorInfo::SyncLinkConflict {
                path: "/home/op/secret".into(),
            },
        ];
        for info in carriers {
            let scrubbed = serde_json::to_string(&sanitize(info.clone())).unwrap();
            assert!(
                !scrubbed.contains("/home/op/secret"),
                "{info:?} still carries the path"
            );
        }
    }

    #[test]
    fn sanitising_leaves_a_semantic_failure_untouched() {
        let info = ErrorInfo::EntryNotFound {
            entry: proto::error::EntryKind::Server,
            reference: "smp".into(),
        };
        assert_eq!(sanitize(info.clone()), info);
    }
}

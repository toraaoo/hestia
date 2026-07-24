/**
 * Mirror of `proto::error::ErrorInfo` — the structured daemon error the
 * front-end localizes. Discriminated by `kind`; token fields are snake_case
 * string unions matching the Rust enums' serialized values.
 */

export type EntryKind = 'server' | 'instance';
export type Nameable = 'server' | 'instance' | 'profile' | 'global_profile';
export type ProfileScope = 'instance' | 'global';

export type Field =
  | 'name'
  | 'project'
  | 'version'
  | 'item'
  | 'backup'
  | 'command'
  | 'program'
  | 'url'
  | 'flavor'
  | 'world'
  | 'memory'
  | 'jvm_args'
  | 'port'
  | 'players'
  | 'backup_interval'
  | 'backup_retention'
  | 'java_version';

export type Reason =
  | 'memory_format'
  | 'jvm_args_prefix'
  | 'port_number'
  | 'port_range'
  | 'whole_number'
  | 'interval_format'
  | 'interval_too_short'
  | 'retention_positive'
  | 'min_players'
  | 'min_backups'
  | 'java_major';

export type Unsupported =
  | 'server_content_kinds'
  | 'vanilla_no_mods'
  | 'worlds_for_datapacks_only'
  | 'datapacks_per_world'
  | 'modpack_not_single_file';

export type Service =
  | 'adoptium'
  | 'mojang'
  | 'fabric'
  | 'modrinth'
  | 'microsoft'
  | 'xbox';

export type Task = 'install' | 'modify' | 'back_up';

export type SyncReason =
  | 'copied_target'
  | 'not_folder_target'
  | 'managed_dir'
  | 'unsafe_path';

export type ErrorInfo =
  | { kind: 'field_required'; field: Field }
  | { kind: 'fields_required'; fields: Field[] }
  | { kind: 'invalid_value'; field: Field; reason: Reason }
  | { kind: 'mutually_exclusive'; options: string[] }
  | { kind: 'nothing_to_do'; what: Task }
  | { kind: 'eula_required' }
  | { kind: 'busy'; detail: string }
  | { kind: 'reserved_name'; name: string }
  | { kind: 'unsupported_operation'; reason: Unsupported }
  | { kind: 'invalid_texture'; detail: string }
  | { kind: 'entry_not_found'; entry: EntryKind; reference: string }
  | { kind: 'process_not_found'; id: string }
  | { kind: 'backup_not_found'; reference: string }
  | { kind: 'content_not_found'; reference: string }
  | { kind: 'profile_not_found'; scope: ProfileScope; name: string }
  | { kind: 'skin_not_found'; key: string }
  | { kind: 'world_not_found'; world: string }
  | { kind: 'account_not_found'; reference: string }
  | { kind: 'version_not_found'; reference: string }
  | { kind: 'config_key_unknown'; key: string }
  | { kind: 'config_key_unset'; key: string }
  | { kind: 'config_type_mismatch'; detail: string }
  | { kind: 'config_rejected'; key: string; detail: string }
  | { kind: 'already_exists'; entry: Nameable; name: string }
  | { kind: 'port_unavailable'; port: number }
  | { kind: 'entry_running'; entry: EntryKind; name: string }
  | { kind: 'not_running'; entry: EntryKind; name: string }
  | { kind: 'provisioning'; name: string }
  | { kind: 'update_in_progress'; name: string }
  | { kind: 'content_in_progress'; name: string }
  | { kind: 'backup_in_progress'; name: string }
  | { kind: 'no_console'; name: string }
  | { kind: 'no_game_port'; name: string }
  | { kind: 'profile_already_captured'; name: string }
  | { kind: 'profile_not_captured'; name: string }
  | { kind: 'sign_in_required' }
  | { kind: 'session_expired'; reference: string }
  | { kind: 'login_declined' }
  | { kind: 'login_timed_out' }
  | { kind: 'not_a_modpack'; reference: string }
  | { kind: 'modpack_invalid'; detail: string }
  | { kind: 'sync_target_invalid'; path: string; reason: SyncReason }
  | { kind: 'sync_link_conflict'; path: string }
  | { kind: 'unknown_channel'; channel: string }
  | { kind: 'malformed_request'; detail: string }
  | { kind: 'io'; operation: string; detail: string }
  | { kind: 'upstream'; service: Service; detail: string }
  | { kind: 'download_failed'; detail: string }
  | { kind: 'rcon_failed'; detail: string }
  | { kind: 'internal'; detail: string };

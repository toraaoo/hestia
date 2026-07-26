/**
 * The structured daemon error surfaced over the socket
 * (`proto::error::ErrorInfo`).
 *
 * The type is **generated** from the Rust enum by ts-rs — run
 * `scripts/gen-types.sh` after changing `crates/proto/src/error.rs` — so it can
 * never drift from the wire. The front-end treats it as opaque: it is rendered
 * generically by `errorMessageFromInfo` (see `../core/errors`) and never
 * inspected field by field, so adding a variant needs only a message key, not a
 * code change here.
 */
export type { ErrorInfo } from './generated/ErrorInfo';

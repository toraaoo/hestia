//! Network reachability.

use proto::net::NetStatus;
use proto::Empty;

use crate::runtime::Channels;

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<NetStatus, _, _>(|_: Empty, ctx| async move {
        Ok(ctx.runtime.engine().network().status())
    });
}

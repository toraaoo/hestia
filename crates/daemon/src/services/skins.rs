//! Skin and cape management for a signed-in account — the desktop's skin
//! picker. Every change relays to Mojang with the account's token, so the
//! handlers stay thin over the engine's skin flows.

use proto::skins::{
    CapeClear, CapeEquip, SkinAdd, SkinAddResult, SkinEquip, SkinList, SkinListResult, SkinRemove,
    SkinReset, SkinUpdate, SkinUpdateResult,
};
use proto::Empty;

use crate::runtime::{Channels, ServiceError};

pub(super) fn register(on: &mut Channels<'_>) {
    on.handle::<SkinList, _, _>(|p, ctx| async move {
        let (skins, capes) = ctx
            .runtime
            .engine()
            .list_skins(&p.account)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(SkinListResult { skins, capes })
    });

    on.handle::<SkinAdd, _, _>(|p, ctx| async move {
        tracing::info!(name = %p.name, variant = ?p.variant, "skin upload started");
        let skin = ctx
            .runtime
            .engine()
            .add_skin(&p.account, &p.name, p.variant, &p.data)
            .await
            .map_err(|e| ServiceError::bad_request(format!("{e:#}")))?;
        Ok(SkinAddResult { skin })
    });

    on.handle::<SkinEquip, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .equip_skin(&p.account, &p.key)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(Empty {})
    });

    on.handle::<SkinReset, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .reset_skin(&p.account)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(Empty {})
    });

    on.handle::<SkinUpdate, _, _>(|p, ctx| async move {
        if ctx.runtime.engine().skins().entry(&p.key).is_none() {
            return Err(ServiceError::not_found(format!(
                "no library skin matches '{}'",
                p.key
            )));
        }
        let skin = ctx
            .runtime
            .engine()
            .update_skin(&p.account, &p.key, &p.name, p.variant)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(SkinUpdateResult { skin })
    });

    on.handle::<SkinRemove, _, _>(|p, ctx| async move {
        match ctx.runtime.engine().skins().remove(&p.key) {
            Ok(true) => Ok(Empty {}),
            Ok(false) => Err(ServiceError::not_found(format!(
                "no library skin matches '{}'",
                p.key
            ))),
            Err(e) => Err(ServiceError::handler_error(format!("{e:#}"))),
        }
    });

    on.handle::<CapeEquip, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .equip_cape(&p.account, &p.cape)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(Empty {})
    });

    on.handle::<CapeClear, _, _>(|p, ctx| async move {
        ctx.runtime
            .engine()
            .clear_cape(&p.account)
            .await
            .map_err(|e| ServiceError::handler_error(format!("{e:#}")))?;
        Ok(Empty {})
    });
}

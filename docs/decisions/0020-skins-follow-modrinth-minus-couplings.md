# Skins follow Modrinth's shape, minus its couplings — and skip the CLI

*Applies to: [Accounts & skins](../architecture/accounts.md)*

Skin management (`skin.*`/`cape.*`) is a desktop-only surface: picking a skin is
visual, so the CLI deliberately grows no command for it. The design mirrors
Modrinth's launcher where its rules earn their keep: a local library preserves
textures (before any change, the currently equipped skin is saved into the
library if neither it nor a default already records it — switching away from an
externally-set skin must never lose it), library rows are keyed by Mojang's
texture hash (an upload response reports the minted key and the row follows it),
and the vanilla defaults are listed by their public texture URLs rather than
bundled PNGs (equipping one is a by-URL skin change). It deliberately drops two
Modrinth couplings: the library is **global**, not per-account (a texture is not
an entitlement; the equipped state is per-account already), and a cape is
**not** bound to a skin — Mojang's own API models them as independent
(`skins/active` vs `capes/active`), and binding them is what forces Modrinth's
save-row reconciliation dance. Changes apply immediately (no debounce): the
daemon is resident, so there is no app-close edge to flush.

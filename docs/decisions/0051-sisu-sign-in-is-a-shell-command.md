# Sign-in is the one bespoke shell command — it must be

*Applies to: [Front-ends](../architecture/frontends.md)*

Microsoft sign-in over the sisu flow (`bridge.rs`'s sibling `commands/auth.rs`,
one `account_login_sisu` command) is the deliberate exception to the
[generic-pipe rule](0049-desktop-bridge-is-one-generic-command.md), for the same
reason Modrinth's launcher makes it one: the flow opens Microsoft's sign-in page
in a **native webview window** and completes only by reading that window's URL
when it redirects to `login.live.com/oauth20_desktop.srf?code=…` — and a
cross-origin webview's location is readable only from the Rust side, never from
the frontend JS that spawned it. So the command drives the two ordinary daemon
calls (`account.login.begin` with the sisu method → `account.login.complete`
with the captured code) around a `WebviewWindowBuilder` window it polls, closing
it on success and returning the stored account (or `null` if the window is
dismissed before completing — a cancel, not an error). The window is frameless
(`decorations(false)`, matching the app shell's chrome-less windows) and carries
no capability entry (the default capability is scoped to `main`), because the
external sign-in page needs no Tauri IPC; only Rust touches it. The device code
flow keeps its plain `account.login.*` path over the generic bridge for the CLI
— the webview dance is a desktop-only affordance layered over the same
contracts, adding no wire surface. The player-head avatar the shell shows for
each account is rendered from the public `mc-heads.net/avatar/<uuid>` service
(helm overlay included, initials fallback), the same source Modrinth uses — the
account list carries only `{uuid, name}`, so the head is derived from the uuid
rather than round-tripped.

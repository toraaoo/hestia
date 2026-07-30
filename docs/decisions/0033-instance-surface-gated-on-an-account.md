# Instances are gated on a signed-in account, in the router

*Applies to: [The daemon](../architecture/daemon.md)*

A user cannot use — let alone play — Minecraft they do not own, and a stored
account already proves ownership (sign-in resolves the game profile). So the
whole instance surface is refused until an account exists: `Router::route`
checks a channel prefix (`instance.*`, plus the instance-only `sync.*`) and
answers a new `unauthorized` code before dispatch when
`accounts().has_account()` is false. The gate lives at the router rather than in
each handler for two reasons: it is a whole-domain lockdown (a per-handler
`require_account` across ~30 handlers is error-prone and easy to forget on the
next channel), and prefixing covers `instance.content.*` and
`instance.profile.*` in their own service modules without touching them.
Catalogue reads shared with servers (`content.*`) and the global `profile.*`
reference lists stay open; only `instance.profile.apply` (instance-prefixed) is
gated. Every front-end inherits the gate for free — the CLI surfaces the
`unauthorized` message, and the desktop pairs it with a route-guard redirect, a
library sign-in overlay, and a first-run prompt.

# A flavor states what it needs, before the user commits to it

*Applies to: [Minecraft providers](../architecture/minecraft.md)*

Spigot and CraftBukkit are the first flavors that can be *unavailable on this
machine*: BuildTools drives git, and bootstraps its own only on Windows. Failing
at create would tell a user who has never heard of git that something went wrong
minutes in, so `Flavor` carries `requires` — the prerequisites resolved as
**missing** when the catalogue is built, each with a name the user would
recognise and where to get it. A front-end renders them beside the flavor
without knowing which flavor needs what; the refusal itself is
`ErrorInfo::MissingRequirement`, the same structured shape. The check is
`Engine`'s, not `Minecraft`'s: whether a tool is installed is a question about
this computer, and the catalogue stays a pure read of the providers.

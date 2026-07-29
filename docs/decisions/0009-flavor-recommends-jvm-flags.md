# A flavor may recommend JVM flags; the user still outranks it

*Applies to: [Minecraft providers](../architecture/minecraft.md)*

PaperMC publishes a tuned G1GC set per version, and running a Paper server
without it is measurably worse — so `ServerProfile` carries `jvm_args` and they
become the last fallback beneath the entry's own `jvm-args` and the
launcher-wide `defaults.jvm-args`. No new mechanism was needed: `or_defaults`
already fills only what a layer left unset, so the flavor chains onto the
existing call. Memory is deliberately excluded — how much RAM to give a server
is the user's call, not a catalogue's.

The cost is flags the user never typed, which `config get jvm-args` would
honestly report as unset while the server ran with eighteen of them. That is the
hidden-behaviour failure this codebase already rejects elsewhere, so `server
info` names the effective flags **and which layer supplied them**
(`JvmArgsSource`). A front-end must not have to guess why a process has flags
nobody set.

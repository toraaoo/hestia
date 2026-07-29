# Stopping the daemon has three meanings; the front-end picks one, the wire carries two

*Applies to: [The daemon](../architecture/daemon.md)*

`daemon.stop` takes a boolean `stop_processes`, but a user typing `hestia daemon
stop` with a server running has not expressed either value — "stop the launcher"
is genuinely ambiguous about the server, and both guesses are bad (killing it
loses the world's unsaved state; keeping it silently leaves a process the user
thinks they stopped). So the *third* meaning — **ask** — lives in the front-end,
not the contract: the CLI prompts on a terminal, and when piped refuses and
names both flags, so a script must say which it meant. With no workloads running
there is nothing to decide and the stop is immediate.

Each front-end therefore declares its meaning rather than inheriting a default:
the CLI asks, the **tray's Quit** stops the daemon and leaves workloads running
(a menu item cannot ask, and quitting a tray icon must not kill someone's
server), and the **desktop's stop button** does the same for the same reason.
The one thing none of them does is decide silently while pretending the wire
default did it. This was drift, not design, until now: the CLI's help still
claimed workloads "keep running unless `--all`", which described neither the
prompt nor the refusal.

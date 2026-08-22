# The tray speaks the tray protocol, not a library that might be installed

*Applies to: [Front-ends](../architecture/frontends.md)*

The Linux status area is a D-Bus protocol — `org.kde.StatusNotifierItem`, hosted
by a watcher the desktop provides. `tray-icon` reaches it through
libappindicator, a *client* library that wraps that protocol and that most
desktops do not ship: a stock Arch or CachyOS install has none, and neither does
SteamOS, whose desktop mode is Plasma and therefore runs a full watcher. So the
icon was absent on machines with a perfectly good tray, and because
libappindicator is dlopened on first use and panics when missing, the daemon
spawning the tray on every serve wrote a crash report each time. [0054's
unconditional spawn](0054-the-daemon-spawns-the-tray.md) made that loud.

Probing for the library first turned the crash into a clean exit, which fixed
the report and not the tray. The library was never the requirement — the
protocol is. Chromium reached the same conclusion and reimplemented its tray as
`StatusIconLinuxDbus`, which is why an Electron app has an icon where we had
none.

The tray now implements the protocol itself, through `ksni` (pure Rust over
`zbus`). What it needs is a session bus and a desktop that hosts a tray; there
is nothing to install, `libayatana-appindicator3-dev` leaves CI and the `.deb`,
and the Linux tray links no GTK at all. A session with no watcher is a normal
state rather than an error: the item stays published and registers when one
appears, so enabling the GNOME extension or restarting a bar brings the icon
back with nothing to restart.

`tray-icon`/`tao` stay on Windows, where the notification area really does want
a window and a message pump. The two backends sit under `platform/`, with the
daemon polling, the menu's vocabulary and the icon raster shared above them —
the worker reports through a `Sink` so it never knows which one is rendering.

**Rejected:** bundling libayatana-appindicator in our own packages — it drags
the GTK3 ABI chain into every artifact and still leaves a source build at the
mercy of the host. Keeping `tray-icon` with a `ksni` fallback — two Linux
backends to maintain for a protocol we would then be speaking twice, with the
GTK dependency retained for the path that works less often. An XEmbed fallback
for pre-SNI panels — Chromium dropped it too, and `snixembed` bridges those
sessions for every app at once rather than each app carrying the code.

# The tray and desktop must not share a GApplication id

*Applies to: [Front-ends](../architecture/frontends.md)*

On Linux both front-ends go through `tao`, which creates a `gtk::Application`
with `ApplicationFlags::empty()` and registers it — so GApplication acquires the
D-Bus name equal to the app id and enforces single-instance by name ownership.
When the tray reused `common::app::ID` (the desktop shell's Tauri `identifier`),
whichever process started second registered as a *remote* instance and never
showed — the tray blocked the desktop and vice versa. The tray now registers
under its own `common::app::TRAY_ID` (`…hestia.tray`), decoupling the two.
Single-instance *within* each front-end is enforced deliberately, not
accidentally: the tray by its [per-endpoint runtime
lock](0054-the-daemon-spawns-the-tray.md), and the desktop by
`tauri-plugin-single-instance` — a second `hestia-desktop` (e.g. the tray's
left-click) hands its args to the running instance and exits, and the plugin
callback shows/unminimises/focuses the existing `main` window. So only one tray
and one desktop ever run, and re-launching surfaces what is already open.

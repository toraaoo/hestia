# Server provisioning is front-loaded by design

*Applies to: [Servers & instances](../architecture/entries.md)*

A server is a long-lived, repeatedly-started thing, often driven
headless/scripted — `create` pays the whole cost once (jar, java, EULA) so
`start` is an immediate spawn that cannot fail on the network. An instance is
the opposite: cheap to create, and its heavyweight files (client jar, shared
libraries, thousands of assets) are ensured idempotently at launch, shared
across instances via the `meta/libraries/` / `meta/assets/` / `meta/versions/`
roots.

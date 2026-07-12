/**
 * The data-access seam: screens consume these hooks and never reach the store
 * or mock fixtures directly. Wiring the daemon (client SDK over Tauri) later
 * replaces the hook internals — the components don't change.
 */
export * from "./account";
export * from "./content";
export * from "./instances";
export * from "./servers";

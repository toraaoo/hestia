export const INSTANCE_TABS = [
  "overview",
  "mods",
  "worlds",
  "screenshots",
  "logs",
  "settings",
] as const;

export type InstanceTab = (typeof INSTANCE_TABS)[number];

export function parseInstanceTab(value: unknown): InstanceTab {
  return INSTANCE_TABS.includes(value as InstanceTab) ? (value as InstanceTab) : "overview";
}

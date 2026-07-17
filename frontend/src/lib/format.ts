/** Display-only formatting helpers for the launcher UI. */

import { m } from '@/paraglide/messages.js';

export function agoLabel(unix: number): string {
  const secs = Math.max(0, Date.now() / 1000 - unix);
  const mins = Math.round(secs / 60);
  if (mins < 1) return m['ago.just_now']();
  if (mins < 60) return m['ago.minutes']({ count: mins });
  const hours = Math.round(mins / 60);
  if (hours < 24) return m['ago.hours']({ count: hours });
  const days = Math.round(hours / 24);
  if (days === 1) return m['ago.yesterday']();
  return m['ago.days']({ count: days });
}

export function bytes(n: number): string {
  const units = ['B', 'KB', 'MB', 'GB'];
  let value = n;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value < 10 && unit > 0 ? 1 : 0)} ${units[unit]}`;
}

/** Gigabytes parsed from a memory setting like `4G` (defaults to 4). */
export function memGb(memory: string): number {
  return Number.parseInt(memory, 10) || 4;
}

export function compact(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return `${n}`;
}

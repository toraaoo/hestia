/**
 * Guards the message catalogue against the two ways it drifts: a locale falling
 * behind the base one, and keys outliving their call sites.
 *
 * Keys reached dynamically (`m[`error.kind.${kind}`]`) have no literal call
 * site, so every such table is declared in `DYNAMIC_PREFIXES` below. Adding a
 * dynamic lookup without listing it here fails the dead-key test — deliberately,
 * since an undeclared table is exactly what makes dead keys unprovable.
 */
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = join(import.meta.dirname, '..', '..');
const MESSAGES = join(ROOT, 'messages');
const BASE_LOCALE = 'en';

const DYNAMIC_PREFIXES = [
  'error.kind.',
  'error.code.',
  'error.token.',
  'warning.kind.',
  'warning.hint.',
  'warning.token.',
  'domain.entry_type.',
  'domain.gamemode.',
  'domain.difficulty.',
  'domain.flavor.',
  'domain.export_format.',
  'domain.import_format.',
];

function flatten(value: unknown, prefix = ''): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, child] of Object.entries(value as object)) {
    if (key === '$schema') continue;
    const path = `${prefix}${key}`;
    if (typeof child === 'object' && child !== null)
      Object.assign(out, flatten(child, `${path}.`));
    else out[path] = child as string;
  }
  return out;
}

function catalogue(locale: string): Record<string, string> {
  const dir = join(MESSAGES, locale);
  const out: Record<string, string> = {};
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')))
    Object.assign(out, flatten(JSON.parse(readFileSync(join(dir, file), 'utf8'))));
  return out;
}

function sources(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'paraglide') continue;
    const path = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...sources(path));
    else if (/\.tsx?$/.test(entry.name)) out.push(readFileSync(path, 'utf8'));
  }
  return out;
}

function placeholders(message: string): string[] {
  return [...message.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
}

const locales = readdirSync(MESSAGES, { withFileTypes: true })
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

const base = catalogue(BASE_LOCALE);
const referenced = new Set(
  sources(join(ROOT, 'src')).flatMap((text) => [
    ...[...text.matchAll(/m(?:sg|essages)?\[\s*['"]([^'"]+)['"]\s*\]/g)].map(
      (match) => match[1],
    ),
  ]),
);

describe('message catalogue', () => {
  it('lists more than one locale', () => {
    expect(locales).toContain(BASE_LOCALE);
    expect(locales.length).toBeGreaterThan(1);
  });

  it.each(locales.filter((l) => l !== BASE_LOCALE))(
    '%s covers the base locale exactly',
    (locale) => {
      const keys = Object.keys(catalogue(locale));
      expect(Object.keys(base).filter((k) => !keys.includes(k))).toEqual([]);
      expect(keys.filter((k) => !(k in base))).toEqual([]);
    },
  );

  it.each(locales.filter((l) => l !== BASE_LOCALE))(
    '%s interpolates the same placeholders',
    (locale) => {
      const translated = catalogue(locale);
      const mismatched = Object.keys(base).filter(
        (key) =>
          key in translated &&
          placeholders(base[key]).join() !== placeholders(translated[key]).join(),
      );
      expect(mismatched).toEqual([]);
    },
  );

  it('defines every key the source references', () => {
    expect([...referenced].filter((key) => !(key in base)).sort()).toEqual([]);
  });

  it('has no message without a call site', () => {
    const dead = Object.keys(base).filter(
      (key) =>
        !referenced.has(key) &&
        !DYNAMIC_PREFIXES.some((prefix) => key.startsWith(prefix)),
    );
    expect(dead.sort()).toEqual([]);
  });
});

import { describe, expect, it } from 'vitest';
import {
  contentKinds,
  isContentKind,
  kindBySlug,
  kindInfo,
  sourceSearch,
} from '@/features/shared/content/lib';
import { isContentUrl } from '@/queries/content';

describe('isContentKind', () => {
  it('admits every kind the wire defines', () => {
    for (const kind of contentKinds) expect(isContentKind(kind)).toBe(true);
  });

  it('refuses anything else a search param could carry', () => {
    expect(isContentKind('mods')).toBe(false);
    expect(isContentKind('')).toBe(false);
    expect(isContentKind(undefined)).toBe(false);
    expect(isContentKind(1)).toBe(false);
  });
});

describe('kindBySlug', () => {
  it('round-trips every kind through its route slug', () => {
    for (const kind of contentKinds) {
      expect(kindBySlug(kindInfo[kind].slug)).toBe(kind);
    }
  });

  it('is undefined for a slug no kind claims', () => {
    expect(kindBySlug('worlds')).toBeUndefined();
  });

  it('gives each kind a distinct slug', () => {
    const slugs = contentKinds.map((kind) => kindInfo[kind].slug);
    expect(new Set(slugs).size).toBe(slugs.length);
  });
});

describe('sourceSearch', () => {
  it('carries a named source through', () => {
    expect(sourceSearch({ source: 'modrinth' })).toEqual({
      source: 'modrinth',
    });
  });

  it('omits the param entirely for the default source', () => {
    expect(sourceSearch({})).toEqual({});
    expect(sourceSearch({ source: '' })).toEqual({});
    expect(sourceSearch({ source: 7 })).toEqual({});
  });
});

describe('isContentUrl', () => {
  it('recognises a pasted project link', () => {
    expect(isContentUrl('https://modrinth.com/mod/sodium')).toBe(true);
    expect(isContentUrl('  http://example.com/x  ')).toBe(true);
  });

  it('treats an ordinary search as a search', () => {
    expect(isContentUrl('sodium')).toBe(false);
    expect(isContentUrl('modrinth.com/mod/sodium')).toBe(false);
    expect(isContentUrl('')).toBe(false);
  });

  it('refuses a link with whitespace inside it', () => {
    expect(isContentUrl('https://example.com/a b')).toBe(false);
  });
});

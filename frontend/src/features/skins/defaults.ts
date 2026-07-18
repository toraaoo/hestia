// One card per character (Modrinth's rule): the canonical model, Steve and
// Alex first, except an equipped non-canonical variant wins its card.
import type { Skin, SkinVariant } from '@/api';

const CANONICAL_MODEL: Record<string, SkinVariant> = {
  Steve: 'classic',
  Alex: 'slim',
  Zuri: 'classic',
  Sunny: 'classic',
  Noor: 'slim',
  Makena: 'slim',
  Kai: 'classic',
  Efe: 'slim',
  Ari: 'classic',
};

const FIRST = ['Steve', 'Alex'];

export function collapseDefaults(skins: Skin[]): Skin[] {
  const byName = new Map<string, Skin[]>();
  for (const skin of skins) {
    if (skin.source !== 'default') continue;
    const group = byName.get(skin.name ?? '') ?? [];
    group.push(skin);
    byName.set(skin.name ?? '', group);
  }

  const collapsed = [...byName.entries()].map(
    ([name, group]) =>
      group.find((s) => s.equipped) ??
      group.find((s) => s.variant === CANONICAL_MODEL[name]) ??
      group[0],
  );
  return collapsed.sort((a, b) => {
    const ai = FIRST.indexOf(a.name ?? '');
    const bi = FIRST.indexOf(b.name ?? '');
    return (ai === -1 ? FIRST.length : ai) - (bi === -1 ? FIRST.length : bi);
  });
}

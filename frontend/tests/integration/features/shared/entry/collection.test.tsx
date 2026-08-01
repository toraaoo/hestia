import { screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import {
  EntryCollection,

  filterCards,
  flavorsOf,
} from '@/features/shared/entry/components';
import type { EntryCardModel } from '@/features/shared/entry/components';
import { renderWithProviders, resetQueryCache } from '../../../../support';

const card = (over: Partial<EntryCardModel> = {}): EntryCardModel => ({
  id: 'a',
  name: 'Alpha',
  kind: 'instance',
  flavor: 'fabric',
  version: '1.21.1',
  running: false,
  ready: true,
  subtitle: 'stopped',
  ...over,
});

const cards = [
  card(),
  card({ id: 'b', name: 'Bravo', flavor: 'neoforge', version: '1.20.1' }),
  card({ id: 'c', name: 'Charlie', flavor: 'fabric', version: '1.21.4' }),
];

afterEach(resetQueryCache);

describe('filterCards', () => {
  it('keeps everything when nothing is asked for', () => {
    expect(filterCards(cards, '')).toHaveLength(3);
  });

  it('matches name, flavor and version, case-insensitively', () => {
    expect(filterCards(cards, 'brav').map((c) => c.id)).toEqual(['b']);
    expect(filterCards(cards, 'FABRIC').map((c) => c.id)).toEqual(['a', 'c']);
    expect(filterCards(cards, '1.20').map((c) => c.id)).toEqual(['b']);
  });

  it('ignores surrounding whitespace', () => {
    expect(filterCards(cards, '  alpha  ').map((c) => c.id)).toEqual(['a']);
  });

  it('narrows by flavor alongside the query', () => {
    expect(filterCards(cards, '', 'neoforge').map((c) => c.id)).toEqual(['b']);
    expect(filterCards(cards, 'charlie', 'neoforge')).toHaveLength(0);
  });

  it('treats "all" as no flavor constraint', () => {
    expect(filterCards(cards, '', 'all')).toHaveLength(3);
  });
});

describe('flavorsOf', () => {
  it('lists each flavor present once', () => {
    expect(flavorsOf(cards)).toEqual(['fabric', 'neoforge']);
  });
});

describe('EntryCollection', () => {
  it('says why it is empty rather than showing a blank grid', async () => {
    renderWithProviders(
      <EntryCollection cards={[]} view="grid" empty="nothing here" />,
      { route: true },
    );
    expect(await screen.findByText('nothing here')).toBeDefined();
  });

  it('renders every entry in grid view', async () => {
    renderWithProviders(
      <EntryCollection cards={cards} view="grid" empty="nothing here" />,
      { route: true },
    );
    for (const entry of cards) {
      expect(await screen.findByText(entry.name)).toBeDefined();
    }
  });

  it('renders every entry in list view too', async () => {
    renderWithProviders(
      <EntryCollection cards={cards} view="list" empty="nothing here" />,
      { route: true },
    );
    for (const entry of cards) {
      expect(await screen.findByText(entry.name)).toBeDefined();
    }
  });
});

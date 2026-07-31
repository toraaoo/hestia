/**
 * `skin.*` / `cape.*` — the account's skin picture: the saved library, the
 * vanilla defaults, and the capes it owns. At most one of each is equipped,
 * which the list is the authority on.
 */
import type { Cape, Skin } from '@/api/types';

import { fail, type Handlers, ok, str } from '../support';
import { textures } from './textures';

const defaults: Skin[] = [
  {
    key: 'default-steve',
    name: 'Steve',
    variant: 'classic',
    texture: textures.classic,
    source: 'default',
    equipped: false,
  },
  {
    key: 'default-alex',
    name: 'Alex',
    variant: 'slim',
    texture: textures.slim,
    source: 'default',
    equipped: false,
  },
];

const library: Skin[] = [
  {
    key: 'library-mock',
    name: 'Mock Skin',
    variant: 'classic',
    texture: textures.library,
    source: 'library',
    equipped: true,
  },
];

const capes: Cape[] = [
  {
    id: 'migrator',
    name: 'Migrator',
    texture: textures.cape,
    equipped: false,
  },
];

const all = (): Skin[] => [...library, ...defaults];

function equip(key: string): void {
  for (const skin of all()) skin.equipped = skin.key === key;
}

export const channels: Handlers = {
  'skin.list': () => ({ skins: all(), capes }),

  'skin.add': (p) => {
    const skin: Skin = {
      key: `library-${library.length + 1}`,
      name: str(p, 'name', 'New Skin'),
      variant: str(p, 'variant', 'classic') === 'slim' ? 'slim' : 'classic',
      // The upload is base64 PNG on the wire; the library serves it back as
      // the data URL the preview renders from.
      texture: `data:image/png;base64,${str(p, 'data')}`,
      source: 'library',
      equipped: true,
    };
    library.unshift(skin);
    equip(skin.key);
    return { skin };
  },

  'skin.update': (p) => {
    const key = str(p, 'key');
    const skin = library.find((entry) => entry.key === key);
    if (!skin) fail('not_found', `no such skin: ${key}`);
    skin.name = str(p, 'name', skin.name);
    skin.variant =
      str(p, 'variant', skin.variant) === 'slim' ? 'slim' : 'classic';
    return { skin };
  },

  'skin.equip': (p) => {
    equip(str(p, 'key'));
    return ok();
  },

  'skin.reset': () => {
    equip('default-steve');
    return ok();
  },

  'skin.remove': (p) => {
    const at = library.findIndex((entry) => entry.key === str(p, 'key'));
    if (at >= 0) library.splice(at, 1);
    return ok();
  },

  'cape.equip': (p) => {
    const id = str(p, 'cape');
    for (const cape of capes) cape.equipped = cape.id === id;
    return ok();
  },

  'cape.clear': () => {
    for (const cape of capes) cape.equipped = false;
    return ok();
  },
};

/**
 * Static stand-in skins and capes, shaped after the Mojang profile surface
 * the skins feature will eventually read. Nothing talks to a backend.
 */

export type SkinVariant = 'classic' | 'slim';

/** A cape the account owns, from the Mojang profile. */
export interface Cape {
  id: string;
  name: string;
  texture: string;
}

export const capes: Cape[] = [
  { id: 'vanilla', name: 'Vanilla', texture: '/capes/vanilla.png' },
  { id: 'home', name: 'Home', texture: '/capes/home.png' },
  {
    id: 'cherry-blossom',
    name: 'Cherry Blossom',
    texture: '/capes/cherry-blossom.png',
  },
];

export const getCape = (id?: string) => capes.find((c) => c.id === id);

/** A skin in the library: bundled defaults plus the user's saved skins. */
export interface Skin {
  id: string;
  name: string;
  variant: SkinVariant;
  /** URL or data URL of the 64x64 skin texture. */
  texture: string;
  cape_id?: string;
  source: 'default' | 'custom';
}

/** Each default character with its canonical model, as the vanilla launcher presents them. */
const defaultSkinModels: [string, SkinVariant][] = [
  ['Steve', 'classic'],
  ['Alex', 'slim'],
  ['Ari', 'classic'],
  ['Efe', 'slim'],
  ['Kai', 'classic'],
  ['Makena', 'slim'],
  ['Noor', 'slim'],
  ['Sunny', 'classic'],
  ['Zuri', 'classic'],
];

export const defaultSkins: Skin[] = defaultSkinModels.map(([name, variant]) => {
  const slug = name.toLowerCase();
  return {
    id: `default-${slug}`,
    name,
    variant,
    texture: `/skins/${slug}${variant === 'slim' ? '-slim' : ''}.png`,
    source: 'default',
  };
});

export const customSkins: Skin[] = [
  {
    id: 'custom-adventurer',
    name: 'Adventurer',
    variant: 'classic',
    texture: '/skins/zuri.png',
    cape_id: 'vanilla',
    source: 'custom',
  },
  {
    id: 'custom-netherborn',
    name: 'Netherborn',
    variant: 'slim',
    texture: '/skins/efe-slim.png',
    cape_id: 'home',
    source: 'custom',
  },
];

/** The skin currently applied to the signed-in account. */
export const equippedSkinId = 'custom-adventurer';

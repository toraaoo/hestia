/** Mirrors `crates/proto/src/skins.rs`. */

export type SkinVariant = 'classic' | 'slim';

/**
 * How the daemon knows about a skin: a vanilla default, a saved library
 * entry, or the account's currently equipped texture that neither covers.
 */
export type SkinSource = 'default' | 'library' | 'external';

export interface Skin {
  /** The texture hash — the stable identity a library row and an equip name. */
  key: string;
  name?: string;
  variant: SkinVariant;
  /** An https texture URL, or a data URL for a library blob. */
  texture: string;
  source: SkinSource;
  equipped: boolean;
}

export interface Cape {
  id: string;
  name: string;
  /** The Mojang-hosted texture URL. */
  texture: string;
  equipped: boolean;
}

export interface SkinList {
  /**
   * Library entries, then the vanilla defaults, then — only when neither
   * covers it — the equipped external skin. At most one entry is `equipped`.
   */
  skins: Skin[];
  /** The capes the account owns; at most one is `equipped`. */
  capes: Cape[];
}

/** A component spreads `anchor(id)`; the step that highlights it names the same id. */
export type TourAnchor =
  | 'nav'
  | 'account'
  | 'daemon-status'
  | 'play-bar'
  | 'page-search'
  | 'page-actions'
  | 'library-new'
  | 'browse-kinds'
  | 'browse-sources'
  | 'entry-run'
  | 'instance-content'
  | 'instance-profiles'
  | 'instance-worlds'
  | 'server-details'
  | 'server-console'
  | 'server-backups'
  | 'skin-preview'
  | 'skin-library';

const ATTRIBUTE = 'data-tour';

export function anchor(id: TourAnchor): Record<string, string> {
  return { [ATTRIBUTE]: id };
}

export function findAnchor(id: TourAnchor): HTMLElement | null {
  return document.querySelector<HTMLElement>(`[${ATTRIBUTE}="${id}"]`);
}

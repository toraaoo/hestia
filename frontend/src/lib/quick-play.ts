/**
 * Whether an instance can join a world or a server on start.
 *
 * The game learned the Quick Play arguments in 1.20; an older client ignores
 * them and opens to the title screen, so the daemon refuses such a launch
 * outright. The same rule lives here so the UI can disable the action and say
 * why, rather than offering a button that only fails once pressed. A version
 * that does not read as a release triple (a snapshot) answers no, matching the
 * daemon's own refusal.
 */
const QUICK_PLAY_SINCE = [1, 20, 0];

export function supportsQuickPlay(gameVersion: string): boolean {
  const parts = gameVersion.trim().split('.');
  if (parts.length < 2) return false;
  const version = parts.map((part) => Number(part));
  if (version.some((part) => !Number.isInteger(part) || part < 0)) return false;
  while (version.length < 3) version.push(0);
  for (let i = 0; i < 3; i++) {
    if (version[i] !== QUICK_PLAY_SINCE[i]) {
      return version[i] > QUICK_PLAY_SINCE[i];
    }
  }
  return true;
}

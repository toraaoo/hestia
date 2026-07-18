/**
 * Static stand-in Java runtimes, shaped after `java.list` / `java.releases`.
 * Nothing talks to a backend.
 */

/** An installed Java runtime, from `java.list`. */
export interface JavaRuntime {
  vendor: string;
  major: number;
  version: string;
  inUse: boolean;
}

export const javaRuntimes: JavaRuntime[] = [
  { vendor: 'Temurin', major: 21, version: '21.0.5+11', inUse: true },
  { vendor: 'Temurin', major: 17, version: '17.0.13+11', inUse: true },
  { vendor: 'Temurin', major: 8, version: '8u432-b06', inUse: false },
];

/** An installable Java major, from `java.releases`. */
export interface JavaRelease {
  major: number;
  lts: boolean;
  installed: boolean;
}

export const javaReleases: JavaRelease[] = [
  { major: 8, lts: true, installed: true },
  { major: 11, lts: true, installed: false },
  { major: 17, lts: true, installed: true },
  { major: 21, lts: true, installed: true },
  { major: 23, lts: false, installed: false },
];

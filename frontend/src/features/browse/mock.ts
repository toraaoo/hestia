/**
 * Static stand-in Modrinth projects, shaped after `content.search` /
 * `content.project`. Nothing talks to a backend.
 */

import type { ContentKind } from '@/lib/mock';

const DAY = 86_400;
const nowSec = Math.floor(Date.now() / 1000);
const daysAgo = (days: number) => nowSec - days * DAY;

/** A Modrinth project, from `content.search` / `content.project`. */
export interface ContentProject {
  id: string;
  title: string;
  author: string;
  description: string;
  kind: ContentKind;
  downloads: number;
  follows: number;
  categories: string[];
  updated_unix: number;
}

export const contentProjects: ContentProject[] = [
  {
    id: 'sodium',
    title: 'Sodium',
    author: 'jellysquid3',
    description:
      'A modern rendering engine that massively improves frame rates and reduces stuttering.',
    kind: 'mod',
    downloads: 42_100_000,
    follows: 78_400,
    categories: ['Optimization'],
    updated_unix: daysAgo(5),
  },
  {
    id: 'iris',
    title: 'Iris Shaders',
    author: 'coderbot',
    description:
      'A modern shaders mod compatible with the OptiFine shader pack ecosystem.',
    kind: 'mod',
    downloads: 28_600_000,
    follows: 61_200,
    categories: ['Optimization'],
    updated_unix: daysAgo(9),
  },
  {
    id: 'fabric-api',
    title: 'Fabric API',
    author: 'modmuss50',
    description:
      'The essential hooks and interoperability mod for the Fabric toolchain.',
    kind: 'mod',
    downloads: 98_300_000,
    follows: 40_900,
    categories: ['Library'],
    updated_unix: daysAgo(2),
  },
  {
    id: 'complementary',
    title: 'Complementary Shaders',
    author: 'EminGT',
    description:
      'A high-quality, well-optimized shader pack with a distinctive look.',
    kind: 'shader',
    downloads: 12_800_000,
    follows: 33_500,
    categories: ['Fantasy', 'Vanilla-like'],
    updated_unix: daysAgo(15),
  },
  {
    id: 'lithium',
    title: 'Lithium',
    author: 'jellysquid3',
    description:
      'A general-purpose optimization mod for the game logic and tick loop.',
    kind: 'mod',
    downloads: 31_500_000,
    follows: 44_100,
    categories: ['Optimization'],
    updated_unix: daysAgo(7),
  },
  {
    id: 'create',
    title: 'Create',
    author: 'simibubi',
    description: 'Aesthetic automation and contraptions with rotational power.',
    kind: 'mod',
    downloads: 21_200_000,
    follows: 52_300,
    categories: ['Technology'],
    updated_unix: daysAgo(30),
  },
  {
    id: 'faithful',
    title: 'Faithful 32x',
    author: 'Faithful Team',
    description:
      'A faithful, higher-resolution take on the default texture pack.',
    kind: 'resourcepack',
    downloads: 9_400_000,
    follows: 18_700,
    categories: ['Vanilla-like'],
    updated_unix: daysAgo(22),
  },
  {
    id: 'voice-chat',
    title: 'Simple Voice Chat',
    author: 'henkelmax',
    description:
      'Proximity voice chat for any server, no extra software required.',
    kind: 'mod',
    downloads: 17_900_000,
    follows: 29_800,
    categories: ['Social'],
    updated_unix: daysAgo(12),
  },
];

export const getProject = (id: string) =>
  contentProjects.find((p) => p.id === id);

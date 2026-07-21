/**
 * The per-entry content mutations, shared by servers and instances. The five
 * operations (`add`/`remove`/`update`/`enable`/`setVersion`) are identical
 * across both kinds bar the entry tag, the api namespace, the key prefixes,
 * and instance's extra `profiles` invalidation — so they live once here and
 * each domain file calls `entryContentFactories` with its own config.
 */
import type { QueryKey } from '@tanstack/react-query';
import type {
  ContentAddSpec,
  ContentDone,
  ContentKind,
  ProvisionProgress,
} from '../api';
import { mutation } from './core';
import { type JobEntryKind, jobMutation } from './jobs';

type OnProgress = (progress: ProvisionProgress) => void;

/** The content api surface both `serverApi.content` and `instanceApi.content` satisfy. */
export interface EntryContentApi {
  add(
    id: string,
    spec: ContentAddSpec,
    onProgress?: OnProgress,
  ): Promise<ContentDone>;
  remove(
    id: string,
    kind: ContentKind,
    item: string,
    worlds?: string[],
  ): Promise<void>;
  update(
    id: string,
    kind: ContentKind,
    item?: string,
    onProgress?: OnProgress,
  ): Promise<ContentDone>;
  enable(
    id: string,
    kind: ContentKind,
    item: string,
    enabled: boolean,
    worlds?: string[],
  ): Promise<void>;
  setVersion(
    id: string,
    kind: ContentKind,
    item: string,
    version: string,
    onProgress?: OnProgress,
  ): Promise<ContentDone>;
}

export interface EntryContentConfig {
  /** The entry tag jobs carry, so an activity surface groups by entry. */
  kind: JobEntryKind;
  api: EntryContentApi;
  /** The entry's content key prefix (`keys.<kind>.content`). */
  contentKey: (id: string) => QueryKey;
  /** The entry's footprint key (`keys.<kind>.info`). */
  infoKey: (id: string) => QueryKey;
  /** Extra prefixes to sweep — instances add their `profiles(id)`. */
  extraInvalidate?: (id: string) => QueryKey[];
}

/** The `content.*` mutation factories for one entry kind. */
export function entryContentFactories(cfg: EntryContentConfig) {
  const invalidates = (id: string): QueryKey[] => [
    cfg.contentKey(id),
    cfg.infoKey(id),
    ...(cfg.extraInvalidate?.(id) ?? []),
  ];
  return {
    /** Refused on a running or busy entry. */
    add: (id: string) =>
      jobMutation<ContentDone, ContentAddSpec>({
        mutationKey: [...cfg.contentKey(id), 'add'],
        meta: (spec) => ({
          kind: 'content.add',
          label: `add ${spec.kind}`,
          entry: { kind: cfg.kind, id },
        }),
        run: (spec, onProgress) => cfg.api.add(id, spec, onProgress),
        invalidates: () => invalidates(id),
      }),
    remove: (id: string) =>
      mutation<void, { kind: ContentKind; item: string; worlds?: string[] }>({
        mutationKey: [...cfg.contentKey(id), 'remove'],
        mutationFn: ({ kind, item, worlds }) =>
          cfg.api.remove(id, kind, item, worlds),
        invalidates: () => invalidates(id),
      }),
    /** `item` empty updates every platform-sourced item of the kind. */
    update: (id: string) =>
      jobMutation<ContentDone, { kind: ContentKind; item?: string }>({
        mutationKey: [...cfg.contentKey(id), 'update'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `update ${kind}s`,
          entry: { kind: cfg.kind, id },
        }),
        run: ({ kind, item }, onProgress) =>
          cfg.api.update(id, kind, item, onProgress),
        invalidates: () => invalidates(id),
      }),
    enable: (id: string) =>
      mutation<
        void,
        { kind: ContentKind; item: string; enabled: boolean; worlds?: string[] }
      >({
        mutationKey: [...cfg.contentKey(id), 'enable'],
        mutationFn: ({ kind, item, enabled, worlds }) =>
          cfg.api.enable(id, kind, item, enabled, worlds),
        invalidates: () => invalidates(id),
      }),
    setVersion: (id: string) =>
      jobMutation<
        ContentDone,
        { kind: ContentKind; item: string; version: string }
      >({
        mutationKey: [...cfg.contentKey(id), 'set-version'],
        meta: ({ kind }) => ({
          kind: 'content.update',
          label: `pin ${kind}`,
          entry: { kind: cfg.kind, id },
        }),
        run: ({ kind, item, version }, onProgress) =>
          cfg.api.setVersion(id, kind, item, version, onProgress),
        invalidates: () => invalidates(id),
      }),
  };
}

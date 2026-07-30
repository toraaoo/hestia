/**
 * `announce.*` — the news and notices fetched from the published feed.
 *
 * The list is the daemon's cache, so it is cheap and works offline; `refresh`
 * is the only call that goes to the network.
 */
import { queryOptions, useMutation, useQuery } from '@tanstack/react-query';
import type { AnnounceListResult, Announcement } from '../api';
import * as api from '../api/announce';
import { mutation } from './core';
import { keys } from './keys';

export const announceQueries = {
  list: () =>
    queryOptions({
      queryKey: keys.announce.list(),
      queryFn: () => api.list(),
    }),
};

export const announceMutations = {
  /** Mark announcements read. */
  dismiss: () =>
    mutation<AnnounceListResult, string[]>({
      mutationKey: [...keys.announce.all, 'dismiss'],
      mutationFn: (ids) => api.dismiss(ids),
      invalidates: () => [keys.announce.all],
    }),
  /** Fetch the feed now instead of waiting for the daemon's poll. */
  refresh: () =>
    mutation<AnnounceListResult, void>({
      mutationKey: [...keys.announce.all, 'refresh'],
      mutationFn: () => api.refresh(),
      invalidates: () => [keys.announce.all],
    }),
};

export function useAnnouncements() {
  return useQuery(announceQueries.list());
}

export function useDismissAnnouncements() {
  return useMutation(announceMutations.dismiss());
}

export function useRefreshAnnouncements() {
  return useMutation(announceMutations.refresh());
}

/** Announcements the user has not marked read, newest first. */
export function unread(result: AnnounceListResult | undefined): Announcement[] {
  return result?.announcements.filter((a) => !a.dismissed) ?? [];
}

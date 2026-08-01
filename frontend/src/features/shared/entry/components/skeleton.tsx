import { CardGridSkeleton } from '@/components/skeleton';

/** Card bones in the entry collection's grid — servers, instances, library. */
export function EntryGridSkeleton({
  count = 8,
  header = false,
}: {
  count?: number;
  header?: boolean;
}) {
  return (
    <CardGridSkeleton
      header={header}
      grid="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4"
      count={count}
      card="h-40"
    />
  );
}

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
      grid="grid gap-3 [grid-template-columns:repeat(auto-fill,minmax(10rem,1fr))]"
      count={count}
      card="aspect-[2/3]"
    />
  );
}

import { Bone, CardGridSkeleton } from '@/components/skeleton';

const SKIN_GRID =
  'grid grid-cols-[repeat(auto-fill,minmax(7.25rem,1fr))] gap-3';

/** The skins page while `skin.list` loads: preview rail plus the card grids. */
export function SkinsPageSkeleton() {
  return (
    <div className="flex items-start gap-6">
      <div className="w-64 shrink-0">
        <Bone className="h-[330px]" />
        <div className="border border-t-0 border-border p-3">
          <Bone className="h-4 w-32" />
          <div className="mt-2 flex gap-1.5">
            <Bone className="h-5 w-14" />
            <Bone className="h-5 w-20" />
          </div>
          <Bone className="mt-3 h-3 w-40" />
        </div>
      </div>

      <div className="min-w-0 flex-1 space-y-8">
        <CardGridSkeleton header grid={SKIN_GRID} count={2} card="h-40" />
        <CardGridSkeleton header grid={SKIN_GRID} count={9} card="h-40" />
      </div>
    </div>
  );
}

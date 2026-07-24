import { UploadSimpleIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';

import {
  type ContentKind,
  type ContentProject,
  content as contentApi,
  dialog,
} from '@/api';
import { contentIcon, contentKindLabel } from '@/components/icons';
import { PickerPanel } from '@/components/picker-panel';
import { projectRef } from '@/features/content/components/content-card';
import { PickRow } from '@/features/content/components/pick-row';
import { kindInfo } from '@/features/content/lib/kinds';
import { m } from '@/paraglide/messages.js';
import { contentQueries } from '@/queries/content';
import { instanceQueries } from '@/queries/instance';

import { FilterBar } from '../filter-bar';
import {
  ACCEPTS,
  fileName,
  type PickedFile,
  type Target,
  targetTakesKind,
  useInstalledRefs,
} from '../targets';

export function ContentStep({
  target,
  kind,
  onKindChange,
  picked,
  onToggle,
  onAddFiles,
}: {
  target: Target;
  kind: ContentKind | null;
  onKindChange: (kind: ContentKind | null) => void;
  picked: ContentProject[];
  onToggle: (p: ContentProject) => void;
  onAddFiles: (files: PickedFile[]) => void;
}) {
  const [search, setSearch] = useState('');
  const kinds = ACCEPTS[target.type].filter((k) => targetTakesKind(target, k));
  // Datapacks land inside a world; an instance with none can take none.
  const worlds = useQuery({
    ...instanceQueries.worlds(target.id),
    enabled: target.type === 'instance',
  });
  const noWorlds = target.type === 'instance' && worlds.data?.length === 0;
  const datapackBlocked = (k: ContentKind) => k === 'data_pack' && noWorlds;
  const activeKind = kind ?? kinds[0];
  const pickedRefs = new Set(picked.map(projectRef));
  const installedRefs = useInstalledRefs(target, activeKind);

  const results = useQuery(
    contentQueries.search({
      kind: activeKind,
      query: search.trim(),
      loader: activeKind === 'mod' ? target.flavor : undefined,
      gameVersion: target.gameVersion || undefined,
      limit: 30,
    }),
  );
  const hits = results.data?.hits ?? [];

  return (
    <PickerPanel
      header={
        <>
          <FilterBar
            search={search}
            onSearch={setSearch}
            placeholder={m['search.modrinth']()}
            chips={kinds.map((k) => ({
              label: kindInfo[k].label(),
              active: activeKind === k,
              disabled: datapackBlocked(k),
              onClick: () => onKindChange(k),
            }))}
          />

          {/* A global profile stores project references, never files. */}
          {target.type !== 'profile' && (
            <FileImportButton onFiles={onAddFiles} />
          )}
        </>
      }
    >
      {datapackBlocked(activeKind) ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.no_worlds_datapack']()}
        </p>
      ) : results.isPending ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['content.installing']()}
        </p>
      ) : hits.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2 p-0.5">
          {hits.map((p) => {
            const installed = installedRefs.has(`${p.source}:${p.id}`);
            return (
              <PickRow
                key={`${p.source}:${p.id}`}
                icon={contentIcon(p.kind)}
                imageUrl={p.iconUrl}
                title={p.title}
                subtitle={`${contentKindLabel[p.kind]()} · ${m['browse.by_author']({ name: p.author })}`}
                badge={installed ? m['content.installed']() : undefined}
                disabled={installed}
                selected={pickedRefs.has(projectRef(p))}
                onSelect={() => onToggle(p)}
              />
            );
          })}
        </div>
      )}
    </PickerPanel>
  );
}

function FileImportButton({
  onFiles,
}: {
  onFiles: (files: PickedFile[]) => void;
}) {
  return (
    <button
      type="button"
      onClick={async () => {
        const paths = await dialog.pickContentFiles();
        if (paths.length === 0) return;
        const files = await Promise.all(
          paths.map(async (path): Promise<PickedFile> => {
            const r = await contentApi.inspect(path);
            return {
              path,
              filename: r.filename || fileName(path),
              kind: r.kind,
              detected: r.kind,
              valid: r.valid,
              reason: r.reason,
            };
          }),
        );
        onFiles(files);
      }}
      className="mb-2 flex w-full items-center gap-3 border border-dashed border-border p-3 text-left outline-none transition-colors hover:bg-muted/60 focus-visible:ring-1 focus-visible:ring-ring"
    >
      <UploadSimpleIcon className="size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm">
          {m['content.import_file']()}
        </span>
        <span className="block truncate text-[11px] text-muted-foreground">
          {m['content.import_file_hint']()}
        </span>
      </span>
    </button>
  );
}

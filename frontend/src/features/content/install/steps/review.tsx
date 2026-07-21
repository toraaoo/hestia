import { XIcon } from '@phosphor-icons/react';
import { useRef } from 'react';

import type { ContentProject, ContentVersion } from '@/api';
import { contentKindLabel } from '@/components/icons';
import { Badge } from '@/components/ui/badge';
import {
  Combobox,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxList,
} from '@/components/ui/combobox';
import { projectRef } from '@/features/content/content-card';
import { agoLabel } from '@/lib/format';
import { m } from '@/paraglide/messages.js';
import { useContentVersions } from '@/queries/content';

import { fileName, type PickedFile, type Target } from '../targets';

export function ReviewStep({
  target,
  picked,
  files,
  versionIds,
  onVersion,
  onRemoveProject,
  onRemoveFile,
  worlds,
}: {
  target: Target | null;
  picked: ContentProject[];
  files: PickedFile[];
  versionIds: Record<string, string>;
  onVersion: (ref: string, id: string) => void;
  onRemoveProject: (p: ContentProject) => void;
  onRemoveFile: (path: string) => void;
  worlds?: string[];
}) {
  const isProfile = target?.type === 'profile';

  return (
    <div className="flex flex-col gap-4 p-1">
      <div className="divide-y divide-border border border-border">
        <ReviewRow label={m['label.target']()} value={target?.name ?? '—'} />
        {worlds && (
          <ReviewRow
            label={m['label.worlds']()}
            value={
              worlds.length ? worlds.join(', ') : m['content.none_selected']()
            }
          />
        )}
      </div>

      <div className="divide-y divide-border border border-border">
        {picked.map((p) => (
          <ReviewItemRow
            key={projectRef(p)}
            target={target}
            project={p}
            isProfile={isProfile}
            versionId={versionIds[projectRef(p)] ?? ''}
            onVersion={(id) => onVersion(projectRef(p), id)}
            onRemove={() => onRemoveProject(p)}
          />
        ))}
        {files.map((f) => (
          <div
            key={f.path}
            className="flex items-center justify-between gap-4 px-3 py-2 text-sm"
          >
            <div className="min-w-0">
              <span className="block truncate">{fileName(f.path)}</span>
              <span className="block truncate text-[11px] text-muted-foreground">
                {m['content.local_file']()}
              </span>
            </div>
            <RemoveButton onClick={() => onRemoveFile(f.path)} />
          </div>
        ))}
      </div>
    </div>
  );
}

function RemoveButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={m['action.remove']()}
      className="flex size-6 shrink-0 items-center justify-center border border-border text-muted-foreground outline-none transition-colors hover:border-destructive/40 hover:bg-destructive/10 hover:text-destructive focus-visible:ring-1 focus-visible:ring-ring"
    >
      <XIcon weight="bold" className="size-3.5" />
    </button>
  );
}

function ReviewItemRow({
  target,
  project,
  isProfile,
  versionId,
  onVersion,
  onRemove,
}: {
  target: Target | null;
  project: ContentProject;
  isProfile: boolean;
  versionId: string;
  onVersion: (id: string) => void;
  onRemove: () => void;
}) {
  const versions = useContentVersions(
    {
      source: project.source,
      project: projectRef(project),
      loader:
        !isProfile && project.kind === 'mod'
          ? (target?.flavor ?? undefined)
          : undefined,
      gameVersion: !isProfile ? target?.gameVersion || undefined : undefined,
    },
    { enabled: !isProfile },
  );
  const list = versions.data ?? [];
  const resolved = list.find((v) => v.id === versionId) ?? list[0];
  const requiredDeps =
    resolved?.dependencies.filter((d) => d.kind === 'required').length ?? 0;

  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
      <div className="min-w-0">
        <span className="block truncate">{project.title}</span>
        <span className="block truncate text-[11px] text-muted-foreground">
          {contentKindLabel[project.kind]()}
          {requiredDeps > 0 &&
            ` · ${m['content.dependencies']({ count: requiredDeps })}`}
        </span>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        {!isProfile && (
          <>
            {resolved && !versionId && (
              <Badge variant="secondary" className="shrink-0">
                {m['label.latest']()}
              </Badge>
            )}
            {resolved && (
              <VersionCombobox
                versions={list}
                value={resolved}
                onChange={(v) =>
                  onVersion(v && v.id !== list[0]?.id ? v.id : '')
                }
              />
            )}
          </>
        )}
        <RemoveButton onClick={onRemove} />
      </div>
    </div>
  );
}

function VersionCombobox({
  versions,
  value,
  onChange,
}: {
  versions: ContentVersion[];
  value: ContentVersion;
  onChange: (version: ContentVersion | null) => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const latestId = versions[0]?.id;
  return (
    <div ref={rootRef} className="contents">
      <Combobox
        items={versions}
        value={value}
        onValueChange={(v: ContentVersion | null) => {
          onChange(v);
          requestAnimationFrame(() =>
            rootRef.current?.querySelector('input')?.blur(),
          );
        }}
        itemToStringLabel={(v: ContentVersion) => v.versionNumber}
        itemToStringValue={(v: ContentVersion) => v.versionNumber}
      >
        <ComboboxInput
          placeholder={m['content.select_version']()}
          className="w-48"
        />
        <ComboboxContent>
          <ComboboxEmpty>{m['content.no_versions']()}</ComboboxEmpty>
          <ComboboxList>
            {(v: ContentVersion) => (
              <ComboboxItem key={v.id} value={v}>
                <div className="flex min-w-0 flex-col">
                  <span className="flex items-center gap-1.5">
                    {v.versionNumber}
                    {v.id === latestId && (
                      <Badge variant="secondary" className="text-[10px]">
                        {m['label.latest']()}
                      </Badge>
                    )}
                    {v.channel !== 'release' && (
                      <Badge
                        variant="outline"
                        className="text-[10px] capitalize"
                      >
                        {v.channel}
                      </Badge>
                    )}
                  </span>
                  <span className="truncate font-mono text-[11px] text-muted-foreground">
                    {v.gameVersions.join(', ')} ·{' '}
                    {agoLabel(Date.parse(v.datePublished) / 1000)}
                  </span>
                </div>
              </ComboboxItem>
            )}
          </ComboboxList>
        </ComboboxContent>
      </Combobox>
    </div>
  );
}

function ReviewRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 px-3 py-2 text-sm">
      <span className="text-xs text-muted-foreground">{label}</span>
      <span className="truncate text-right">{value}</span>
    </div>
  );
}

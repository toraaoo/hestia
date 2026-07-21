import { useState } from 'react';

import type { ContentKind } from '@/api';
import { contentKindLabel, entryIcon } from '@/components/icons';
import { PickerPanel } from '@/components/picker-panel';
import { PickRow } from '@/features/content/pick-row';
import { m } from '@/paraglide/messages.js';

import { FilterBar } from '../filter-bar';
import { entryTypeLabel, type Target } from '../targets';

export function TargetStep({
  kind,
  targets,
  selectedId,
  onSelect,
}: {
  kind: ContentKind;
  targets: Target[];
  selectedId: string;
  onSelect: (t: Target) => void;
}) {
  const [search, setSearch] = useState('');
  const q = search.trim().toLowerCase();
  const shown = targets.filter((t) => !q || t.name.toLowerCase().includes(q));

  if (targets.length === 0) {
    return (
      <p className="px-1 py-8 text-center text-xs text-muted-foreground">
        {m['content.no_target_for_kind']({
          kind: contentKindLabel[kind]().toLowerCase(),
        })}
      </p>
    );
  }
  return (
    <PickerPanel
      header={
        <FilterBar
          search={search}
          onSearch={setSearch}
          placeholder={m['search.targets']()}
        />
      }
    >
      {shown.length === 0 ? (
        <p className="px-1 py-8 text-center text-xs text-muted-foreground">
          {m['browse.nothing_matches']()}
        </p>
      ) : (
        <div className="grid gap-2 p-0.5">
          {shown.map((t) => (
            <PickRow
              key={t.id}
              icon={entryIcon(t.type)}
              title={t.name}
              subtitle={
                t.type === 'profile'
                  ? entryTypeLabel(t.type)
                  : `${entryTypeLabel(t.type)} · ${t.flavor} · ${t.gameVersion}`
              }
              badge={t.running ? m['content.stop_to_install']() : undefined}
              disabled={t.running}
              selected={selectedId === t.id}
              onSelect={() => onSelect(t)}
            />
          ))}
        </div>
      )}
    </PickerPanel>
  );
}

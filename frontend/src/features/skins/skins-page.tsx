import { PlusIcon } from '@phosphor-icons/react';
import { useQuery } from '@tanstack/react-query';
import { useRef, useState } from 'react';

import type { Skin } from '@/api';
import { Empty } from '@/components/empty';
import { Page, Section } from '@/components/page';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { CapeCard, CapeGrid } from '@/features/skins/cape-card';
import { collapseDefaults } from '@/features/skins/defaults';
import type { SkinDraft } from '@/features/skins/edit-skin-modal';
import { EditSkinModal } from '@/features/skins/edit-skin-modal';
import { PreviewPanel } from '@/features/skins/preview-panel';
import { SkinsPageSkeleton } from '@/features/skins/skeleton';
import {
  SkinCard,
  SkinGrid,
  skinDisplayName,
} from '@/features/skins/skin-card';
import { readTextureFile } from '@/features/skins/texture';
import { m } from '@/paraglide/messages.js';
import {
  skinQueries,
  useAccounts,
  useAddSkin,
  useClearCape,
  useEquipCape,
  useEquipSkin,
  useRemoveSkin,
  useUpdateSkin,
} from '@/queries';

const BASE64_PREFIX = /^data:image\/png;base64,/;

export function SkinsPage() {
  const accounts = useAccounts();
  const signedIn = (accounts.data?.accounts.length ?? 0) > 0;
  const list = useQuery({ ...skinQueries.list(''), enabled: signedIn });

  const add = useAddSkin();
  const update = useUpdateSkin();
  const equip = useEquipSkin();
  const removeSkin = useRemoveSkin();
  const equipCape = useEquipCape();
  const clearCape = useClearCape();

  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [modal, setModal] = useState<{
    skin: Skin | null;
    texture?: string;
  } | null>(null);
  const [pendingRemove, setPendingRemove] = useState<Skin | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const skins = list.data?.skins ?? [];
  const capes = list.data?.capes ?? [];
  const saved = skins.filter((s) => s.source !== 'default');
  const defaults = collapseDefaults(skins);
  const shown = [...saved, ...defaults];
  const equipped = shown.find((s) => s.equipped);
  const equippedCape = capes.find((c) => c.equipped);
  const selected =
    (selectedKey && shown.find((s) => s.key === selectedKey)) ||
    equipped ||
    shown[0];
  const previewing = selected != null && selected.key !== equipped?.key;
  const capeBusy = equipCape.isPending || clearCape.isPending;

  const addFromFile = async (file: File | undefined) => {
    if (!file?.type.includes('png')) return;
    add.reset();
    setModal({ skin: null, texture: await readTextureFile(file) });
  };

  const openEdit = (skin: Skin) => {
    update.reset();
    setModal({ skin });
  };

  const saveDraft = (draft: SkinDraft) => {
    const editing = modal?.skin;
    if (editing) {
      update.mutate(
        { key: editing.key, name: draft.name, variant: draft.variant },
        { onSuccess: () => setModal(null) },
      );
    } else {
      add.mutate(
        {
          name: draft.name,
          variant: draft.variant,
          data: draft.texture.replace(BASE64_PREFIX, ''),
        },
        {
          onSuccess: (skin) => {
            setModal(null);
            setSelectedKey(skin.key);
          },
        },
      );
    }
  };

  const body = !signedIn ? (
    <Empty>{m['skins.sign_in_hint']()}</Empty>
  ) : list.isError ? (
    <Empty>
      {m['skins.load_failed']()}
      <span className="mt-1 block font-mono text-[11px]">
        {list.error.message}
      </span>
    </Empty>
  ) : (
    <div className="flex items-start gap-6">
      {selected && (
        <PreviewPanel
          skin={selected}
          cape={equippedCape}
          previewing={previewing}
          applying={equip.isPending}
          error={equip.error?.message}
          onApply={() => equip.mutate({ key: selected.key })}
        />
      )}

      <div className="min-w-0 flex-1 space-y-8">
        <Section title={m['skins.your_skins']()} count={saved.length}>
          {saved.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {m['skins.none_yet']()}
            </p>
          ) : (
            <SkinGrid>
              {saved.map((skin) => (
                <SkinCard
                  key={skin.key}
                  skin={skin}
                  selected={skin.key === selected?.key}
                  equipped={skin.equipped}
                  onSelect={() => setSelectedKey(skin.key)}
                  onEquip={() => equip.mutate({ key: skin.key })}
                  onEdit={
                    skin.source === 'library' ? () => openEdit(skin) : undefined
                  }
                  onRemove={
                    skin.source === 'library'
                      ? () => setPendingRemove(skin)
                      : undefined
                  }
                />
              ))}
            </SkinGrid>
          )}
        </Section>

        <Section title={m['skins.default_skins']()} count={defaults.length}>
          <SkinGrid>
            {defaults.map((skin) => (
              <SkinCard
                key={skin.key}
                skin={skin}
                selected={skin.key === selected?.key}
                equipped={skin.equipped}
                onSelect={() => setSelectedKey(skin.key)}
                onEquip={() => equip.mutate({ key: skin.key })}
              />
            ))}
          </SkinGrid>
        </Section>

        <Section title={m['skins.capes']()} count={capes.length}>
          {capes.length === 0 ? (
            <p className="text-xs text-muted-foreground">
              {m['skins.no_capes']()}
            </p>
          ) : (
            <>
              <CapeGrid>
                <CapeCard
                  label={m['skins.no_cape']()}
                  equipped={equippedCape == null}
                  disabled={capeBusy}
                  onEquip={() => clearCape.mutate(undefined)}
                />
                {capes.map((cape) => (
                  <CapeCard
                    key={cape.id}
                    label={cape.name}
                    texture={cape.texture}
                    equipped={cape.equipped}
                    disabled={capeBusy}
                    onEquip={() => equipCape.mutate({ cape: cape.id })}
                  />
                ))}
              </CapeGrid>
              {(equipCape.error || clearCape.error) && (
                <p className="mt-2 text-xs break-words text-destructive">
                  {(equipCape.error ?? clearCape.error)?.message}
                </p>
              )}
            </>
          )}
        </Section>
      </div>
    </div>
  );

  return (
    <Page
      title={m['nav.skins']()}
      subtitle={m['skins.subtitle']()}
      skeleton={<SkinsPageSkeleton />}
      loading={accounts.isPending || (signedIn && list.isPending)}
      actions={
        <>
          <input
            ref={fileRef}
            type="file"
            accept="image/png"
            className="hidden"
            onChange={(e) => {
              addFromFile(e.target.files?.[0]);
              e.target.value = '';
            }}
          />
          <Button
            size="sm"
            data-icon="inline-start"
            disabled={!signedIn}
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={() => fileRef.current?.click()}
          >
            <PlusIcon weight="bold" />
            {m['skins.add']()}
          </Button>
        </>
      }
    >
      {body}

      <EditSkinModal
        open={modal !== null}
        onOpenChange={(open) => {
          if (!open) setModal(null);
        }}
        skin={modal?.skin ?? null}
        initialTexture={modal?.texture}
        saving={add.isPending || update.isPending}
        error={(modal?.skin ? update.error : add.error)?.message}
        onSave={saveDraft}
      />

      <ConfirmDialog
        open={pendingRemove !== null}
        onOpenChange={(open) => !open && setPendingRemove(null)}
        title={m['skins.delete_title']()}
        description={
          pendingRemove &&
          m['skins.delete_description']({
            name: skinDisplayName(pendingRemove),
          })
        }
        destructive
        confirmLabel={m['action.delete']()}
        onConfirm={() => {
          if (pendingRemove) removeSkin.mutate(pendingRemove.key);
          setPendingRemove(null);
        }}
      />
    </Page>
  );
}

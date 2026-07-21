import { PlusIcon } from '@phosphor-icons/react';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useRef, useState } from 'react';

import type { Skin } from '@/api';
import { Empty } from '@/components/empty';
import { Page, Section } from '@/components/page';
import { SignInGate } from '@/components/sign-in-gate';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { CapeCard, CapeGrid } from '@/features/skins/components/cape-card';
import { PreviewPanel } from '@/features/skins/components/preview-panel';
import { SkinsPageSkeleton } from '@/features/skins/components/skeleton';
import {
  SkinCard,
  SkinGrid,
  skinDisplayName,
} from '@/features/skins/components/skin-card';
import type { SkinDraft } from '@/features/skins/edit-modal';
import { EditSkinModal } from '@/features/skins/edit-modal';
import { collapseDefaults } from '@/features/skins/lib/defaults';
import { readTextureFile } from '@/features/skins/lib/texture';
import { m } from '@/paraglide/messages.js';
import { useAccounts } from '@/queries';
import { skinMutations, skinQueries } from '@/queries/skins';

const BASE64_PREFIX = /^data:image\/png;base64,/;

export function SkinsPage() {
  const { signedIn, isPending: accountsPending } = useAccounts();
  const list = useQuery({ ...skinQueries.list(''), enabled: signedIn });

  const add = useMutation(skinMutations.add());
  const update = useMutation(skinMutations.update());
  const equip = useMutation(skinMutations.equip());
  const removeSkin = useMutation(skinMutations.remove());
  const equipCape = useMutation(skinMutations.equipCape());
  const clearCape = useMutation(skinMutations.clearCape());

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
  // Resolve against the full wire list: the variant toggle can select a
  // default's sibling entry that the collapsed grid does not show.
  const selected =
    (selectedKey && skins.find((s) => s.key === selectedKey)) ||
    equipped ||
    shown[0];
  const isSelectedCard = (skin: Skin) =>
    selected != null &&
    (skin.key === selected.key ||
      (skin.source === 'default' &&
        selected.source === 'default' &&
        skin.name === selected.name));
  const previewing = selected != null && selected.key !== equipped?.key;

  const selectVariant = (variant: Skin['variant']) => {
    const sibling = skins.find(
      (s) =>
        s.source === 'default' &&
        s.name === selected?.name &&
        s.variant === variant,
    );
    if (sibling) setSelectedKey(sibling.key);
  };

  const applyCape = (capeId?: string) => {
    if (capeId === equippedCape?.id) return;
    if (capeId) equipCape.mutate({ cape: capeId });
    else clearCape.mutate(undefined);
  };

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
        {
          onSuccess: () => {
            setModal(null);
            applyCape(draft.capeId);
          },
        },
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
            applyCape(draft.capeId);
          },
        },
      );
    }
  };

  const body = !signedIn ? (
    <SignInGate
      title={m['skins.locked_title']()}
      hint={m['skins.sign_in_hint']()}
    />
  ) : list.isError && !list.data ? (
    <Empty>{m['skins.load_failed']()}</Empty>
  ) : (
    <div className="flex items-start gap-6">
      {selected && (
        <PreviewPanel
          skin={selected}
          cape={equippedCape}
          previewing={previewing}
          onApply={() => equip.mutate({ key: selected.key })}
          onVariantChange={
            selected.source === 'default' ? selectVariant : undefined
          }
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
                  selected={isSelectedCard(skin)}
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
                selected={isSelectedCard(skin)}
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
            <CapeGrid>
              <CapeCard
                label={m['skins.no_cape']()}
                equipped={equippedCape == null}
                onEquip={() => clearCape.mutate(undefined)}
              />
              {capes.map((cape) => (
                <CapeCard
                  key={cape.id}
                  label={cape.name}
                  texture={cape.texture}
                  equipped={cape.equipped}
                  onEquip={() => equipCape.mutate({ cape: cape.id })}
                />
              ))}
            </CapeGrid>
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
      loading={accountsPending || (signedIn && list.isPending)}
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
        capes={capes}
        equippedCapeId={equippedCape?.id}
        saving={add.isPending || update.isPending}
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
          if (pendingRemove) removeSkin.mutate({ key: pendingRemove.key });
          setPendingRemove(null);
        }}
      />
    </Page>
  );
}

import { PlusIcon } from '@phosphor-icons/react';
import { useRef, useState } from 'react';

import { Page, Section } from '@/components/page';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import type { SkinDraft } from '@/features/skins/edit-skin-modal';
import { EditSkinModal } from '@/features/skins/edit-skin-modal';
import type { Skin } from '@/features/skins/mock';
import {
  customSkins,
  defaultSkins,
  equippedSkinId,
} from '@/features/skins/mock';
import { PreviewPanel } from '@/features/skins/preview-panel';
import { SkinsPageSkeleton } from '@/features/skins/skeleton';
import { SkinCard, SkinGrid } from '@/features/skins/skin-card';
import { readTextureFile } from '@/features/skins/texture';
import { m } from '@/paraglide/messages.js';

export function SkinsPage() {
  const [custom, setCustom] = useState<Skin[]>(customSkins);
  const [equippedId, setEquippedId] = useState(equippedSkinId);
  const [selectedId, setSelectedId] = useState(equippedSkinId);
  const [modal, setModal] = useState<{
    skin: Skin | null;
    texture?: string;
  } | null>(null);
  const [pendingRemove, setPendingRemove] = useState<Skin | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  const library = [...custom, ...defaultSkins];
  const selected = library.find((s) => s.id === selectedId) ?? library[0];
  const previewing = selected.id !== equippedId;

  const addFromFile = async (file: File | undefined) => {
    if (!file?.type.includes('png')) return;
    setModal({ skin: null, texture: await readTextureFile(file) });
  };

  const saveDraft = (draft: SkinDraft) => {
    if (modal?.skin) {
      const id = modal.skin.id;
      setCustom((skins) =>
        skins.map((s) => (s.id === id ? { ...s, ...draft } : s)),
      );
    } else {
      const skin: Skin = {
        id: `custom-${Date.now().toString(16)}`,
        source: 'custom',
        ...draft,
      };
      setCustom((skins) => [skin, ...skins]);
      setSelectedId(skin.id);
    }
  };

  const equip = (id: string) => {
    setEquippedId(id);
    setSelectedId(id);
  };

  const removeSkin = (id: string) => {
    setCustom((skins) => skins.filter((s) => s.id !== id));
    if (selectedId === id) setSelectedId(equippedId);
    if (equippedId === id) {
      setEquippedId(defaultSkins[0].id);
      if (selectedId === id) setSelectedId(defaultSkins[0].id);
    }
  };

  return (
    <Page
      title={m['nav.skins']()}
      subtitle={m['skins.subtitle']()}
      skeleton={<SkinsPageSkeleton />}
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
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={() => fileRef.current?.click()}
          >
            <PlusIcon weight="bold" />
            {m['skins.add']()}
          </Button>
        </>
      }
    >
      <div className="flex items-start gap-6">
        <PreviewPanel
          skin={selected}
          previewing={previewing}
          onApply={() => setEquippedId(selected.id)}
        />

        <div className="min-w-0 flex-1 space-y-8">
          <Section title={m['skins.your_skins']()} count={custom.length}>
            {custom.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                {m['skins.none_yet']()}
              </p>
            ) : (
              <SkinGrid>
                {custom.map((skin) => (
                  <SkinCard
                    key={skin.id}
                    skin={skin}
                    selected={skin.id === selected.id}
                    equipped={skin.id === equippedId}
                    onSelect={() => setSelectedId(skin.id)}
                    onEquip={() => equip(skin.id)}
                    onEdit={() => setModal({ skin })}
                    onRemove={() => setPendingRemove(skin)}
                  />
                ))}
              </SkinGrid>
            )}
          </Section>

          <Section
            title={m['skins.default_skins']()}
            count={defaultSkins.length}
          >
            <SkinGrid>
              {defaultSkins.map((skin) => (
                <SkinCard
                  key={skin.id}
                  skin={skin}
                  selected={skin.id === selected.id}
                  equipped={skin.id === equippedId}
                  onSelect={() => setSelectedId(skin.id)}
                  onEquip={() => equip(skin.id)}
                />
              ))}
            </SkinGrid>
          </Section>
        </div>
      </div>

      <EditSkinModal
        open={modal !== null}
        onOpenChange={(open) => {
          if (!open) setModal(null);
        }}
        skin={modal?.skin ?? null}
        initialTexture={modal?.texture}
        onSave={saveDraft}
      />

      <ConfirmDialog
        open={pendingRemove !== null}
        onOpenChange={(open) => !open && setPendingRemove(null)}
        title={m['skins.delete_title']()}
        description={
          pendingRemove &&
          m['skins.delete_description']({ name: pendingRemove.name })
        }
        destructive
        confirmLabel={m['action.delete']()}
        onConfirm={() => {
          if (pendingRemove) removeSkin(pendingRemove.id);
          setPendingRemove(null);
        }}
      />
    </Page>
  );
}

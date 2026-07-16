import {
  ArrowCounterClockwiseIcon,
  CheckIcon,
  DotsThreeIcon,
  PencilSimpleIcon,
  PlusIcon,
  TrashIcon,
} from '@phosphor-icons/react';
import { createFileRoute } from '@tanstack/react-router';
import { useRef, useState } from 'react';

import type { SkinDraft } from '@/components/launcher/edit-skin-modal';
import { EditSkinModal } from '@/components/launcher/edit-skin-modal';
import { Page, Section } from '@/components/launcher/page';
import { SkinBody, SkinModel } from '@/components/launcher/skin-render';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import type { Skin } from '@/lib/mock';
import { customSkins, defaultSkins, equippedSkinId, getCape } from '@/lib/mock';
import { readTextureFile } from '@/lib/skin';
import { cn } from '@/lib/utils';

export const Route = createFileRoute('/_app/skins/')({
  component: SkinsPage,
});

function SkinsPage() {
  const [custom, setCustom] = useState<Skin[]>(customSkins);
  const [equippedId, setEquippedId] = useState(equippedSkinId);
  const [selectedId, setSelectedId] = useState(equippedSkinId);
  const [modal, setModal] = useState<{
    skin: Skin | null;
    texture?: string;
  } | null>(null);
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
      title="Skins"
      subtitle="Your Minecraft character"
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
            Add skin
          </Button>
        </>
      }
    >
      <div className="flex items-start gap-6">
        <PreviewPanel
          skin={selected}
          previewing={previewing}
          onApply={() => setEquippedId(selected.id)}
          onReset={() => setSelectedId(equippedId)}
        />

        <div className="min-w-0 flex-1 space-y-8">
          <Section title="Your skins" count={custom.length}>
            {custom.length === 0 ? (
              <p className="text-xs text-muted-foreground">
                No saved skins yet — add one from a PNG texture, or start from a
                default skin.
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
                    onRemove={() => removeSkin(skin.id)}
                  />
                ))}
              </SkinGrid>
            )}
          </Section>

          <Section title="Default skins" count={defaultSkins.length}>
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
    </Page>
  );
}

/** The sticky left panel: animated model plus the preview → apply flow. */
function PreviewPanel({
  skin,
  previewing,
  onApply,
  onReset,
}: {
  skin: Skin;
  previewing: boolean;
  onApply: () => void;
  onReset: () => void;
}) {
  const cape = getCape(skin.cape_id);

  return (
    <div className="sticky top-5 w-64 shrink-0">
      <div className="relative border border-border bg-muted/40">
        {previewing && (
          <Badge
            variant="secondary"
            className="absolute top-2 left-2 z-10 bg-background/80 backdrop-blur-sm"
          >
            Previewing
          </Badge>
        )}
        <SkinModel
          texture={skin.texture}
          capeTexture={cape?.texture}
          variant={skin.variant}
          width={254}
          height={330}
        />
      </div>

      <div className="border border-t-0 border-border p-3">
        <div className="truncate text-sm font-medium">{skin.name}</div>
        <div className="mt-1.5 flex items-center gap-1.5">
          <Badge variant="secondary">
            {skin.variant === 'slim' ? 'Slim' : 'Wide'}
          </Badge>
          <Badge variant="outline">{cape ? cape.name : 'No cape'}</Badge>
        </div>

        <div className="mt-3 flex gap-1.5">
          {previewing ? (
            <>
              <Button
                size="sm"
                data-icon="inline-start"
                className="flex-1 bg-ember text-ember-foreground hover:bg-ember/90"
                onClick={onApply}
              >
                <CheckIcon weight="bold" />
                Apply
              </Button>
              <Button
                size="sm"
                variant="outline"
                data-icon="inline-start"
                onClick={onReset}
              >
                <ArrowCounterClockwiseIcon />
                Reset
              </Button>
            </>
          ) : (
            <p className="text-xs text-muted-foreground">
              This skin is equipped on your account.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function SkinGrid({ children }: { children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(7.25rem,1fr))] gap-3">
      {children}
    </div>
  );
}

function SkinCard({
  skin,
  selected,
  equipped,
  onSelect,
  onEquip,
  onEdit,
  onRemove,
}: {
  skin: Skin;
  selected: boolean;
  equipped: boolean;
  onSelect: () => void;
  onEquip: () => void;
  onEdit?: () => void;
  onRemove?: () => void;
}) {
  return (
    <div
      className={cn(
        'group relative border transition-colors',
        equipped
          ? 'border-ember'
          : selected
            ? 'border-foreground/30 hover:border-ember/40'
            : 'border-border hover:border-ember/40',
      )}
    >
      <button
        type="button"
        onClick={onSelect}
        aria-pressed={selected}
        className="block w-full outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:ring-inset"
      >
        <div className="flex h-28 items-center justify-center bg-muted/40 pt-2">
          <SkinBody
            texture={skin.texture}
            variant={skin.variant}
            className="h-24"
          />
        </div>
        <div className="border-t border-border p-2 text-left">
          <div className="truncate text-xs font-medium">{skin.name}</div>
          <div className="mt-0.5 font-mono text-[10px] text-muted-foreground">
            {skin.variant === 'slim' ? 'Slim' : 'Wide'}
            {skin.cape_id ? ` · ${getCape(skin.cape_id)?.name}` : ''}
          </div>
        </div>
      </button>

      <DropdownMenu>
        <DropdownMenuTrigger
          render={
            <Button
              variant="secondary"
              size="icon-sm"
              aria-label="Skin actions"
              className="absolute top-1.5 right-1.5 bg-background/80 opacity-0 backdrop-blur-sm transition-opacity group-hover:opacity-100 focus-visible:opacity-100 aria-expanded:opacity-100"
            >
              <DotsThreeIcon weight="bold" className="size-3.5" />
            </Button>
          }
        />
        <DropdownMenuContent align="start">
          <DropdownMenuItem disabled={equipped} onClick={onEquip}>
            <CheckIcon />
            {equipped ? 'Equipped' : 'Equip'}
          </DropdownMenuItem>
          {onEdit && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={onEdit}>
                <PencilSimpleIcon />
                Edit
              </DropdownMenuItem>
              {onRemove && (
                <DropdownMenuItem variant="destructive" onClick={onRemove}>
                  <TrashIcon />
                  Delete
                </DropdownMenuItem>
              )}
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  );
}

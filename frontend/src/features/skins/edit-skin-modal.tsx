import { UploadSimpleIcon } from '@phosphor-icons/react';
import { useEffect, useRef, useState } from 'react';
import type { Cape, Skin, SkinVariant } from '@/api';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import { CapeCard, CapeGrid } from '@/features/skins/cape-card';
import { SkinModel } from '@/features/skins/skin-render';
import { readTextureFile } from '@/features/skins/texture';
import { m } from '@/paraglide/messages.js';

export interface SkinDraft {
  name: string;
  variant: SkinVariant;
  /** Add mode only: the texture data URL to upload. */
  texture: string;
  /** The account cape to wear; `undefined` clears it. */
  capeId?: string;
}

/**
 * Add/edit a skin over a live model preview. Add mode arrives with the picked
 * file's texture already loaded and uploads it; edit mode renames a saved
 * entry and picks its arm style — the texture is the entry's identity, so
 * replacing it means adding a new skin. The cape choice edits the *account's*
 * equipped cape (capes are never stored per skin), applied on save.
 */
export function EditSkinModal({
  open,
  onOpenChange,
  skin,
  initialTexture,
  capes,
  equippedCapeId,
  saving,
  error,
  onSave,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The skin being edited, or `null` when adding a new one. */
  skin: Skin | null;
  /** Add mode: the texture data URL read from the picked file. */
  initialTexture?: string;
  /** The account's owned capes; empty hides the cape choice. */
  capes: Cape[];
  equippedCapeId?: string;
  saving: boolean;
  error?: string;
  onSave: (draft: SkinDraft) => void;
}) {
  const [name, setName] = useState('');
  const [variant, setVariant] = useState<SkinVariant>('classic');
  const [texture, setTexture] = useState('');
  const [capeId, setCapeId] = useState<string | undefined>(undefined);
  const fileRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setName(skin?.name ?? '');
    setVariant(skin?.variant ?? 'classic');
    setTexture(skin?.texture ?? initialTexture ?? '');
    setCapeId(equippedCapeId);
  }, [open, skin, initialTexture, equippedCapeId]);

  const adding = skin === null;
  const canSave = texture !== '' && name.trim() !== '' && !saving;

  const pickTexture = async (file: File | undefined) => {
    if (!file?.type.includes('png')) return;
    setTexture(await readTextureFile(file));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {adding ? m['skins.add']() : m['skins.edit']()}
          </DialogTitle>
          <DialogDescription>
            {adding
              ? m['skins.add_description']()
              : m['skins.edit_description']()}
          </DialogDescription>
        </DialogHeader>

        <div className="flex gap-4">
          {/* biome-ignore lint/a11y/noStaticElementInteractions: drop target
              only — the upload button beside it is the accessible path. */}
          <div
            className="grid w-48 shrink-0 place-items-center self-stretch bg-muted/40 ring-1 ring-border"
            onDragOver={(e) => e.preventDefault()}
            onDrop={(e) => {
              e.preventDefault();
              if (adding) pickTexture(e.dataTransfer.files[0]);
            }}
          >
            {texture ? (
              <SkinModel
                texture={texture}
                capeTexture={capes.find((c) => c.id === capeId)?.texture}
                variant={variant}
                width={192}
                height={288}
              />
            ) : (
              <span className="px-4 text-center text-xs text-muted-foreground">
                {m['skins.upload_hint']()}
              </span>
            )}
          </div>

          <div className="flex min-w-0 flex-1 flex-col gap-4">
            <Field>
              <FieldLabel htmlFor="skin-name">{m['label.name']()}</FieldLabel>
              <Input
                id="skin-name"
                value={name}
                placeholder={m['skins.name_placeholder']()}
                onChange={(e) => setName(e.target.value)}
              />
            </Field>

            {adding && (
              <Field>
                <FieldLabel>{m['skins.texture']()}</FieldLabel>
                <input
                  ref={fileRef}
                  type="file"
                  accept="image/png"
                  className="hidden"
                  onChange={(e) => {
                    pickTexture(e.target.files?.[0]);
                    e.target.value = '';
                  }}
                />
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="w-fit"
                  data-icon="inline-start"
                  onClick={() => fileRef.current?.click()}
                >
                  <UploadSimpleIcon />
                  {texture
                    ? m['skins.replace_texture']()
                    : m['skins.upload_texture']()}
                </Button>
              </Field>
            )}

            <Field>
              <FieldLabel>{m['skins.arm_style']()}</FieldLabel>
              <ToggleGroup
                variant="outline"
                size="sm"
                value={[variant]}
                onValueChange={(vals: string[]) => {
                  const next = vals[vals.length - 1];
                  if (next) setVariant(next as SkinVariant);
                }}
              >
                <ToggleGroupItem value="classic">
                  {m['skins.wide']()}
                </ToggleGroupItem>
                <ToggleGroupItem value="slim">
                  {m['skins.slim']()}
                </ToggleGroupItem>
              </ToggleGroup>
            </Field>

            {capes.length > 0 && (
              <Field>
                <FieldLabel>{m['skins.cape']()}</FieldLabel>
                <CapeGrid>
                  <CapeCard
                    label={m['skins.no_cape']()}
                    equipped={capeId === undefined}
                    onEquip={() => setCapeId(undefined)}
                  />
                  {capes.map((cape) => (
                    <CapeCard
                      key={cape.id}
                      label={cape.name}
                      texture={cape.texture}
                      equipped={capeId === cape.id}
                      onEquip={() => setCapeId(cape.id)}
                    />
                  ))}
                </CapeGrid>
              </Field>
            )}

            {error && (
              <p className="text-xs break-words text-destructive">{error}</p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button
            variant="outline"
            disabled={saving}
            onClick={() => onOpenChange(false)}
          >
            {m['action.cancel']()}
          </Button>
          <Button
            disabled={!canSave}
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={() =>
              onSave({ name: name.trim(), variant, texture, capeId })
            }
          >
            {saving
              ? m['skins.saving']()
              : adding
                ? m['skins.add']()
                : m['skins.save']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

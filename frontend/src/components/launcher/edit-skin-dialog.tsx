import { UploadSimpleIcon, XIcon } from '@phosphor-icons/react';
import { useRef, useState } from 'react';

import { CapeFront, SkinModel } from '@/components/launcher/skin-render';
import { Button } from '@/components/ui/button';
import {
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { ToggleGroup, ToggleGroupItem } from '@/components/ui/toggle-group';
import type { WindowDialogContentProps } from '@/dialogs/window-dialog';
import { windowDialog } from '@/dialogs/window-dialog';
import type { Cape, Skin, SkinVariant } from '@/lib/mock';
import { capes, getCape } from '@/lib/mock';
import { readTextureFile } from '@/lib/skin';
import { cn } from '@/lib/utils';

export interface EditSkinPayload {
  /** The skin being edited, or `null` when adding a new one. */
  skin: Skin | null;
  /** Add mode: the texture data URL read from the picked file. */
  texture?: string;
}

export interface SkinDraft {
  name: string;
  variant: SkinVariant;
  texture: string;
  cape_id?: string;
}

/**
 * Add/edit a skin: texture upload, arm style, cape and name over a live
 * model preview. Add mode arrives with the picked file's texture already
 * loaded; edit mode starts from the existing skin.
 */
function EditSkinContent({
  payload,
  close,
}: WindowDialogContentProps<EditSkinPayload, SkinDraft>) {
  const { skin } = payload;
  const [name, setName] = useState(skin?.name ?? '');
  const [variant, setVariant] = useState<SkinVariant>(
    skin?.variant ?? 'classic',
  );
  const [texture, setTexture] = useState(
    skin?.texture ?? payload.texture ?? '',
  );
  const [capeId, setCapeId] = useState<string | undefined>(skin?.cape_id);
  const fileRef = useRef<HTMLInputElement>(null);

  const adding = skin === null;
  const canSave = texture !== '' && name.trim() !== '';

  const pickTexture = async (file: File | undefined) => {
    if (!file?.type.includes('png')) return;
    setTexture(await readTextureFile(file));
  };

  return (
    <>
      <DialogHeader>
        <DialogTitle>{adding ? 'Add skin' : 'Edit skin'}</DialogTitle>
        <DialogDescription>
          {adding
            ? 'Name the skin, pick its arm style and cape.'
            : 'Changes apply the next time you equip this skin.'}
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
            pickTexture(e.dataTransfer.files[0]);
          }}
        >
          {texture ? (
            <SkinModel
              texture={texture}
              capeTexture={getCape(capeId)?.texture}
              variant={variant}
              width={192}
              height={288}
            />
          ) : (
            <span className="px-4 text-center text-xs text-muted-foreground">
              Upload a texture to preview it
            </span>
          )}
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <Field>
            <FieldLabel htmlFor="skin-name">Name</FieldLabel>
            <Input
              id="skin-name"
              value={name}
              placeholder="My skin"
              onChange={(e) => setName(e.target.value)}
            />
          </Field>

          <Field>
            <FieldLabel>Texture</FieldLabel>
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
              {texture ? 'Replace texture' : 'Upload texture'}
            </Button>
          </Field>

          <Field>
            <FieldLabel>Arm style</FieldLabel>
            <ToggleGroup
              variant="outline"
              size="sm"
              value={[variant]}
              onValueChange={(vals: string[]) => {
                const next = vals[vals.length - 1];
                if (next) setVariant(next as SkinVariant);
              }}
            >
              <ToggleGroupItem value="classic">Wide</ToggleGroupItem>
              <ToggleGroupItem value="slim">Slim</ToggleGroupItem>
            </ToggleGroup>
          </Field>

          <Field>
            <FieldLabel>Cape</FieldLabel>
            <div className="grid grid-cols-4 gap-1.5">
              <CapeOption
                label="None"
                selected={capeId === undefined}
                onSelect={() => setCapeId(undefined)}
              >
                <XIcon className="size-5 text-muted-foreground" />
              </CapeOption>
              {capes.map((cape: Cape) => (
                <CapeOption
                  key={cape.id}
                  label={cape.name}
                  selected={capeId === cape.id}
                  onSelect={() => setCapeId(cape.id)}
                >
                  <CapeFront texture={cape.texture} className="h-12" />
                </CapeOption>
              ))}
            </div>
          </Field>
        </div>
      </div>

      <DialogFooter>
        <Button variant="outline" onClick={() => close()}>
          Cancel
        </Button>
        <Button
          disabled={!canSave}
          className="bg-ember text-ember-foreground hover:bg-ember/90"
          onClick={() =>
            close({ name: name.trim(), variant, texture, cape_id: capeId })
          }
        >
          {adding ? 'Add skin' : 'Save skin'}
        </Button>
      </DialogFooter>
    </>
  );
}

export const EditSkinDialog = windowDialog('edit-skin', EditSkinContent, {
  title: (payload) => (payload.skin ? 'Edit skin' : 'Add skin'),
  contentClassName: 'sm:max-w-2xl',
});

function CapeOption({
  label,
  selected,
  onSelect,
  children,
}: {
  label: string;
  selected: boolean;
  onSelect: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      className={cn(
        'flex flex-col items-center gap-1.5 px-1 pt-2.5 pb-1.5 ring-1 transition-colors outline-none focus-visible:ring-ring',
        selected
          ? 'bg-muted ring-ember'
          : 'ring-border hover:bg-muted/60 hover:ring-foreground/20',
      )}
    >
      <span className="grid h-12 place-items-center">{children}</span>
      <span className="w-full truncate text-center text-[10px] text-muted-foreground">
        {label}
      </span>
    </button>
  );
}

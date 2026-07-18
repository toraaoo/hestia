import { CheckIcon } from '@phosphor-icons/react';

import type { Cape, Skin } from '@/api';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { skinDisplayName, skinVariantLabel } from '@/features/skins/skin-card';
import { SkinModel } from '@/features/skins/skin-render';
import { m } from '@/paraglide/messages.js';

/**
 * The sticky left panel: animated model plus the preview → apply flow. The
 * cape is the account's equipped one — account-level, worn with every skin.
 */
export function PreviewPanel({
  skin,
  cape,
  previewing,
  applying,
  error,
  onApply,
}: {
  skin: Skin;
  cape?: Cape;
  previewing: boolean;
  applying: boolean;
  error?: string;
  onApply: () => void;
}) {
  return (
    <div className="sticky top-5 w-64 shrink-0">
      <div className="relative border border-border bg-muted/40">
        {previewing && (
          <Badge
            variant="secondary"
            className="absolute top-2 left-2 z-10 bg-background/80 backdrop-blur-sm"
          >
            {m['skins.previewing']()}
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
        <div className="truncate text-sm font-medium">
          {skinDisplayName(skin)}
        </div>
        <div className="mt-1.5 flex items-center gap-1.5">
          <Badge variant="secondary">{skinVariantLabel(skin)}</Badge>
          <Badge variant="outline">
            {cape ? cape.name : m['skins.no_cape']()}
          </Badge>
        </div>

        <div className="mt-3 flex gap-1.5">
          {previewing ? (
            <Button
              size="sm"
              data-icon="inline-start"
              disabled={applying}
              className="flex-1 bg-ember text-ember-foreground hover:bg-ember/90"
              onClick={onApply}
            >
              <CheckIcon weight="bold" />
              {applying ? m['skins.applying']() : m['action.apply']()}
            </Button>
          ) : (
            <p className="text-xs text-muted-foreground">
              {m['skins.equipped_hint']()}
            </p>
          )}
        </div>
        {error && (
          <p className="mt-2 text-xs break-words text-destructive">{error}</p>
        )}
      </div>
    </div>
  );
}

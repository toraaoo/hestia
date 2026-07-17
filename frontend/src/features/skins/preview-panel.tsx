import { CheckIcon } from '@phosphor-icons/react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { Skin } from '@/features/skins/mock';
import { getCape } from '@/features/skins/mock';
import { SkinModel } from '@/features/skins/skin-render';
import { m } from '@/paraglide/messages.js';

/** The sticky left panel: animated model plus the preview → apply flow. */
export function PreviewPanel({
  skin,
  previewing,
  onApply,
}: {
  skin: Skin;
  previewing: boolean;
  onApply: () => void;
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
        <div className="truncate text-sm font-medium">{skin.name}</div>
        <div className="mt-1.5 flex items-center gap-1.5">
          <Badge variant="secondary">
            {skin.variant === 'slim' ? m['skins.slim']() : m['skins.wide']()}
          </Badge>
          <Badge variant="outline">
            {cape ? cape.name : m['skins.no_cape']()}
          </Badge>
        </div>

        <div className="mt-3 flex gap-1.5">
          {previewing ? (
            <Button
              size="sm"
              data-icon="inline-start"
              className="flex-1 bg-ember text-ember-foreground hover:bg-ember/90"
              onClick={onApply}
            >
              <CheckIcon weight="bold" />
              {m['action.apply']()}
            </Button>
          ) : (
            <p className="text-xs text-muted-foreground">
              {m['skins.equipped_hint']()}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * The wizard's icon field control: a preview of the rolled/generated icon with
 * Randomize and Customize. The value is form-owned (`details.icon`); the draft
 * has no entry to bake into until creation succeeds.
 */
import { ArrowsClockwiseIcon, SparkleIcon } from '@phosphor-icons/react';
import { useState } from 'react';

import type { IconConfig } from '@/api/icons';
import { Button } from '@/components/ui/button';
import { m } from '@/paraglide/messages.js';

import { backgroundStyle, randomIconConfig, symbolOption } from './catalog';
import { IconEditorDialog } from './editor-dialog';

export function IconDraftControl({
  value,
  onChange,
}: {
  value: IconConfig;
  onChange: (config: IconConfig) => void;
}) {
  const [editorOpen, setEditorOpen] = useState(false);
  const symbol = symbolOption(value.symbol);

  return (
    <div className="flex items-center gap-4">
      <div
        role="img"
        aria-label={m['entry.create.icon']()}
        className="relative aspect-square size-20 shrink-0 overflow-hidden ring-1 ring-border"
        style={backgroundStyle(value.background)}
      >
        {symbol && (
          <img src={symbol.asset} alt="" className="size-full object-cover" />
        )}
      </div>
      <div className="flex flex-col gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => onChange(randomIconConfig(value))}
        >
          <ArrowsClockwiseIcon />
          {m['entry.icon.editor.randomize']()}
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => setEditorOpen(true)}
        >
          <SparkleIcon />
          {m['entry.icon.customize']()}
        </Button>
      </div>
      <IconEditorDialog
        open={editorOpen}
        onOpenChange={setEditorOpen}
        initial={value}
        onSaved={onChange}
      />
    </div>
  );
}

/** Compose a generated icon: preview background + symbol, randomize, bake or report. */
import { ArrowsClockwiseIcon, CheckIcon } from '@phosphor-icons/react';
import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';

import type { IconConfig } from '@/api/icons';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Spinner } from '@/components/ui/spinner';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';
import { iconMutations } from '@/queries/icons';

import {
  backgroundIdOf,
  backgroundOption,
  backgroundOptions,
  backgroundStyle,
  DEFAULT_BACKGROUND_ID,
  DEFAULT_SYMBOL_ID,
  randomIconConfig,
  symbolBytes,
  symbolOption,
  symbolOptions,
} from './catalog';

export function IconEditorDialog({
  open,
  onOpenChange,
  entryId,
  initial,
  onSaved,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Present: bake the PNG for this entry on save; absent: only report. */
  entryId?: string;
  initial?: IconConfig | null;
  onSaved?: (config: IconConfig) => void;
}) {
  const [backgroundId, setBackgroundId] = useState(DEFAULT_BACKGROUND_ID);
  const [symbolId, setSymbolId] = useState(DEFAULT_SYMBOL_ID);
  const [saving, setSaving] = useState(false);
  const generate = useMutation(iconMutations.generate());

  useEffect(() => {
    if (!open) return;
    setBackgroundId(
      backgroundIdOf(initial?.background) ?? DEFAULT_BACKGROUND_ID,
    );
    setSymbolId(
      initial?.symbol && symbolOption(initial.symbol)
        ? initial.symbol
        : DEFAULT_SYMBOL_ID,
    );
    setSaving(false);
  }, [open, initial]);

  const selectedBackground = backgroundOption(backgroundId);
  const selectedSymbol = symbolOption(symbolId);
  const config = useMemo<IconConfig>(
    () => ({
      background: {
        ...(selectedBackground?.background ?? backgroundOptions[0].background),
      },
      symbol: symbolId,
    }),
    [selectedBackground, symbolId],
  );

  const saveIcon = async () => {
    if (saving || !selectedSymbol) return;
    setSaving(true);
    try {
      const bytes = await symbolBytes(selectedSymbol.asset);
      if (entryId)
        await generate.mutateAsync({ entryId, config, symbolBytes: bytes });
      onSaved?.(config);
      onOpenChange(false);
    } finally {
      setSaving(false);
    }
  };

  const surprise = () => {
    const rolled = randomIconConfig(config);
    const background = backgroundIdOf(rolled.background);
    if (background) setBackgroundId(background);
    setSymbolId(rolled.symbol);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[880px]">
        <DialogHeader>
          <DialogTitle>{m['entry.icon.editor.title']()}</DialogTitle>
          <DialogDescription>
            {m['entry.icon.editor.description']()}
          </DialogDescription>
        </DialogHeader>

        <div className="grid gap-4 sm:grid-cols-[190px_1fr]">
          <aside className="flex flex-col gap-3 border border-border p-3">
            <div
              role="img"
              aria-label={`${selectedBackground?.name() ?? ''} · ${selectedSymbol?.name() ?? ''}`}
              className="relative aspect-square overflow-hidden ring-1 ring-border"
              style={backgroundStyle(config.background)}
            >
              {selectedSymbol && (
                <img
                  src={selectedSymbol.asset}
                  alt=""
                  className="size-full object-cover"
                />
              )}
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={surprise}
            >
              <ArrowsClockwiseIcon />
              {m['entry.icon.editor.randomize']()}
            </Button>
          </aside>

          <div className="flex min-w-0 flex-col gap-4 overflow-y-auto">
            <section>
              <h3 className="mb-2 font-medium">
                {m['entry.icon.editor.background']()}
              </h3>
              <div className="grid grid-cols-6 gap-2">
                {backgroundOptions.map((option) => (
                  <button
                    type="button"
                    key={option.id}
                    aria-label={option.name()}
                    aria-pressed={backgroundId === option.id}
                    onClick={() => setBackgroundId(option.id)}
                    className={cn(
                      'relative aspect-square ring-1 ring-border outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring',
                      backgroundId === option.id &&
                        'ring-2 ring-foreground/70 ring-offset-1 ring-offset-popover',
                    )}
                    style={backgroundStyle(option.background)}
                  >
                    {backgroundId === option.id && (
                      <CheckIcon className="absolute top-1 right-1 size-4 text-white drop-shadow" />
                    )}
                  </button>
                ))}
              </div>
            </section>

            <section className="min-h-0">
              <h3 className="mb-2 font-medium">
                {m['entry.icon.editor.symbol']()}
              </h3>
              <div className="grid grid-cols-6 gap-2">
                {symbolOptions.map((option) => (
                  <button
                    type="button"
                    key={option.id}
                    aria-label={option.name()}
                    aria-pressed={symbolId === option.id}
                    onClick={() => setSymbolId(option.id)}
                    className={cn(
                      'relative aspect-square overflow-hidden bg-muted/50 ring-1 ring-border outline-none transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring',
                      symbolId === option.id &&
                        'ring-2 ring-foreground/70 ring-offset-1 ring-offset-popover',
                    )}
                  >
                    <img
                      src={option.asset}
                      alt=""
                      className="size-full object-cover"
                    />
                    {symbolId === option.id && (
                      <CheckIcon className="absolute top-1 right-1 size-4 text-foreground" />
                    )}
                  </button>
                ))}
              </div>
            </section>
          </div>
        </div>

        <DialogFooter className="gap-2">
          <Button
            type="button"
            variant="ghost"
            onClick={() => onOpenChange(false)}
            disabled={saving}
          >
            {m['app.action.cancel']()}
          </Button>
          <Button
            type="button"
            className="bg-ember text-ember-foreground hover:bg-ember/90"
            onClick={saveIcon}
            disabled={saving || !selectedSymbol}
          >
            {saving ? <Spinner /> : null}
            {m['entry.icon.editor.save']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

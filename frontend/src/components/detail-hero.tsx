import type { Icon } from '@phosphor-icons/react';
import { CaretRightIcon } from '@phosphor-icons/react';
import { Link, type LinkProps } from '@tanstack/react-router';
import type { ReactNode } from 'react';

/** Breadcrumb + banner hero: parent link, big icon, name, badges, actions. */
export function DetailHero({
  parentLabel,
  parentTo,
  parentParams,
  icon: Icon,
  iconUrl,
  iconAction,
  name,
  badges,
  actions,
}: {
  parentLabel: string;
  parentTo: LinkProps['to'];
  parentParams?: LinkProps['params'];
  icon: Icon;
  /** A remote icon (a content project's) shown in place of the glyph. */
  iconUrl?: string;
  /** An overlay control on the icon tile, revealed on hover. */
  iconAction?: ReactNode;
  name: string;
  badges: ReactNode;
  actions: ReactNode;
}) {
  return (
    <div className="border-b border-border">
      <div className="flex items-center gap-1.5 px-5 py-2 text-xs text-muted-foreground">
        <Link
          to={parentTo}
          params={parentParams}
          className="hover:text-foreground"
        >
          {parentLabel}
        </Link>
        <CaretRightIcon className="size-3" />
        <span className="text-foreground">{name}</span>
      </div>

      <div className="flex items-end gap-4 bg-muted/25 px-5 pt-8 pb-5">
        <span className="group relative grid size-16 shrink-0 place-items-center overflow-hidden bg-muted text-muted-foreground ring-1 ring-border">
          {iconUrl ? (
            <img src={iconUrl} alt="" className="size-full object-cover" />
          ) : (
            <Icon className="size-8" />
          )}
          {iconAction && (
            <span className="absolute right-1 bottom-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
              {iconAction}
            </span>
          )}
        </span>
        <div className="min-w-0 flex-1">
          <h1 className="truncate font-heading text-xl font-semibold">
            {name}
          </h1>
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {badges}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">{actions}</div>
      </div>
    </div>
  );
}

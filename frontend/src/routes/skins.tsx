import { createFileRoute } from "@tanstack/react-router";
import { TopBar } from "@/components/TopBar";
import { UserIcon } from "@/components/icons";

export const Route = createFileRoute("/skins")({
  component: Skins,
});

/** Placeholder — the daemon has no skins feature yet; the nav slot matches the approved design. */
function Skins() {
  return (
    <>
      <TopBar title="Skins" />
      <div className="flex min-h-0 flex-1 items-center justify-center">
        <div className="flex flex-col items-center gap-3.5 pb-16 text-center">
          <div className="flex size-16 items-center justify-center rounded-lg bg-surface-2 text-text-3 shadow-card-flat">
            <UserIcon size={28} />
          </div>
          <span className="font-pixel text-sm tracking-wide text-text-2 uppercase font-crisp">
            Coming soon
          </span>
          <p className="max-w-70 text-sm leading-normal text-text-3">
            Skin management isn't here yet. Your current skin still applies in game.
          </p>
        </div>
      </div>
    </>
  );
}

import { useState } from "react";
import { TopBar } from "@/components/layout/TopBar";
import { SectionHeading } from "@/components/ui/SectionHeading";
import { CheckLabel, Field, RangeInput, Select, TextInput } from "@/components/ui/form";
import logoEmber from "@/assets/brand/logo-ember.svg";

export function SettingsScreen() {
  const [memory, setMemory] = useState(8);

  return (
    <>
      <TopBar title="Settings" />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="px-6 pt-5 pb-10">
          <div className="flex max-w-160 flex-col gap-5">
            <SectionHeading title="General" className="mb-0" />
            <div className="grid grid-cols-2 gap-4">
              <Field label="Theme">
                <Select value="Dark (Hearth)" />
              </Field>
              <Field label="Language">
                <Select value="English (US)" />
              </Field>
            </div>
            <Field label="Default game directory" hint="Where new instances are created.">
              <TextInput defaultValue="~/.hestia/instances" className="font-mono" />
            </Field>

            <SectionHeading title="Java & Performance" className="mt-7 mb-0" />
            <Field
              label="Java runtime"
              hint="Auto-managed. Hestia downloads the right JDK per instance."
            >
              <Select value="Adoptium Temurin 21 (bundled)" />
            </Field>
            <Field
              label={`Default allocated memory — ${memory} GB`}
              hint="Your system has 32 GB. Instances can override this individually."
            >
              <RangeInput
                min={2}
                max={24}
                step={1}
                value={memory}
                onChange={(e) => setMemory(Number(e.target.value))}
              />
            </Field>
            <Field label="Default JVM arguments">
              <TextInput
                defaultValue="-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200"
                className="font-mono"
              />
            </Field>

            <SectionHeading title="On launch" className="mt-7 mb-0" />
            <CheckLabel defaultChecked>Keep the launcher open while a game runs</CheckLabel>
            <CheckLabel>Close the launcher when a game starts</CheckLabel>
            <CheckLabel defaultChecked>Check for mod updates on startup</CheckLabel>

            <div className="mt-1 flex items-center gap-2.5 border-t border-border-2 pt-4.5 text-xs text-text-3">
              <img src={logoEmber} alt="" className="size-4.5 rounded-xs" />
              Hestia 0.0.1 · latest
            </div>
          </div>
        </div>
      </div>
    </>
  );
}

import { useState } from "react";
import { Button } from "@/components/ui/Button";
import { Field, RangeInput, Select, TextInput } from "@/components/ui/form";
import { TrashIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

export function SettingsTab() {
  const instance = useCurrentInstance();
  const [memory, setMemory] = useState(instance.memoryGb);

  return (
    <div className="flex max-w-160 flex-col gap-5">
      <Field label="Instance name">
        <TextInput defaultValue={instance.name} />
      </Field>
      <div className="grid grid-cols-2 gap-4">
        <Field label="Minecraft version">
          <Select value={instance.version} />
        </Field>
        <Field label="Mod loader">
          <Select value={`${instance.loader} 0.16.14`} />
        </Field>
      </div>
      <Field
        label={`Allocated memory — ${memory} GB`}
        hint="Recommended: 6 GB for this modpack. Your system has 32 GB."
      >
        <RangeInput
          min={2}
          max={16}
          step={1}
          value={memory}
          onChange={(e) => setMemory(Number(e.target.value))}
        />
      </Field>
      <Field label="Java arguments">
        <TextInput defaultValue="-XX:+UseG1GC -XX:+ParallelRefProcEnabled" className="font-mono" />
      </Field>
      <div className="mt-0.5 border-t border-border-2 pt-4.5">
        <Button variant="danger">
          <TrashIcon size={15} /> Delete instance
        </Button>
      </div>
    </div>
  );
}

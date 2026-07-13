import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, useWatch } from "react-hook-form";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { TrashIcon } from "@/components/icons";
import { useCurrentInstance } from "./current";

const VERSIONS = ["1.21.1", "1.20.6", "1.20.4", "1.20.1", "1.19.2"];

const schema = z.object({
  name: z.string().min(1, "Instance name is required"),
  version: z.string(),
  loader: z.string(),
  memory: z.number().min(2).max(16),
  javaArgs: z.string(),
});

type Values = z.infer<typeof schema>;

export function SettingsTab() {
  const instance = useCurrentInstance();
  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: instance.name,
      version: instance.version,
      loader: `${instance.loader} 0.16.14`,
      memory: instance.memoryGb,
      javaArgs: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled",
    },
  });
  const memory = useWatch({ control: form.control, name: "memory" });

  const onSubmit = (_values: Values) => {
    // Persisted once instance settings are wired to the daemon.
  };

  return (
    <Form {...form}>
      <form
        className="flex max-w-160 flex-col gap-5"
        onSubmit={(e) => void form.handleSubmit(onSubmit)(e)}
      >
        <FormField
          control={form.control}
          name="name"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Instance name</FormLabel>
              <FormControl>
                <Input {...field} />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
        <div className="grid grid-cols-2 gap-4">
          <FormField
            control={form.control}
            name="version"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Minecraft version</FormLabel>
                <Select value={field.value} onValueChange={field.onChange}>
                  <FormControl>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                  </FormControl>
                  <SelectContent>
                    {VERSIONS.map((v) => (
                      <SelectItem key={v} value={v}>
                        {v}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </FormItem>
            )}
          />
          <FormField
            control={form.control}
            name="loader"
            render={({ field }) => (
              <FormItem>
                <FormLabel>Mod loader</FormLabel>
                <Select value={field.value} onValueChange={field.onChange}>
                  <FormControl>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                  </FormControl>
                  <SelectContent>
                    <SelectItem value={field.value}>{field.value}</SelectItem>
                  </SelectContent>
                </Select>
              </FormItem>
            )}
          />
        </div>
        <FormField
          control={form.control}
          name="memory"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Allocated memory — {memory} GB</FormLabel>
              <FormControl>
                <Slider
                  min={2}
                  max={16}
                  step={1}
                  value={[field.value]}
                  onValueChange={(v) => field.onChange(Array.isArray(v) ? v[0] : v)}
                />
              </FormControl>
              <FormDescription>
                Recommended: 6 GB for this modpack. Your system has 32 GB.
              </FormDescription>
            </FormItem>
          )}
        />
        <FormField
          control={form.control}
          name="javaArgs"
          render={({ field }) => (
            <FormItem>
              <FormLabel>Java arguments</FormLabel>
              <FormControl>
                <Input className="font-mono" {...field} />
              </FormControl>
            </FormItem>
          )}
        />
        <div className="mt-0.5 border-t border-border-2 pt-4.5">
          <Button type="button" variant="danger">
            <TrashIcon size={15} /> Delete instance
          </Button>
        </div>
      </form>
    </Form>
  );
}

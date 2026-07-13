import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, useWatch } from "react-hook-form";
import { z } from "zod";
import { TopBar } from "@/components/layout/top-bar";
import { SectionHeading } from "@/components/ui/section-heading";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
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
import logoEmber from "@/assets/brand/logo-ember.svg";

const THEMES = ["Dark (Hearth)", "Dark (Ink)", "Light"];
const LANGUAGES = ["English (US)", "English (UK)", "Deutsch", "Español"];
const RUNTIMES = ["Adoptium Temurin 21 (bundled)", "Adoptium Temurin 17", "System default"];

const schema = z.object({
  theme: z.string(),
  language: z.string(),
  gameDir: z.string(),
  javaRuntime: z.string(),
  memory: z.number().min(2).max(24),
  jvmArgs: z.string(),
  keepOpen: z.boolean(),
  closeOnStart: z.boolean(),
  checkUpdates: z.boolean(),
});

type Values = z.infer<typeof schema>;

export function SettingsScreen() {
  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: {
      theme: "Dark (Hearth)",
      language: "English (US)",
      gameDir: "~/.hestia/instances",
      javaRuntime: "Adoptium Temurin 21 (bundled)",
      memory: 8,
      jvmArgs: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200",
      keepOpen: true,
      closeOnStart: false,
      checkUpdates: true,
    },
  });
  const memory = useWatch({ control: form.control, name: "memory" });

  const onSubmit = (_values: Values) => {
    // Persisted once app settings are wired to the daemon.
  };

  return (
    <>
      <TopBar title="Settings" />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="px-6 pt-5 pb-10">
          <Form {...form}>
            <form
              className="flex max-w-160 flex-col gap-5"
              onSubmit={(e) => void form.handleSubmit(onSubmit)(e)}
            >
              <SectionHeading title="General" className="mb-0" />
              <div className="grid grid-cols-2 gap-4">
                <FormField
                  control={form.control}
                  name="theme"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Theme</FormLabel>
                      <Select value={field.value} onValueChange={field.onChange}>
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {THEMES.map((t) => (
                            <SelectItem key={t} value={t}>
                              {t}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </FormItem>
                  )}
                />
                <FormField
                  control={form.control}
                  name="language"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>Language</FormLabel>
                      <Select value={field.value} onValueChange={field.onChange}>
                        <FormControl>
                          <SelectTrigger className="w-full">
                            <SelectValue />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {LANGUAGES.map((l) => (
                            <SelectItem key={l} value={l}>
                              {l}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </FormItem>
                  )}
                />
              </div>
              <FormField
                control={form.control}
                name="gameDir"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Default game directory</FormLabel>
                    <FormControl>
                      <Input className="font-mono" {...field} />
                    </FormControl>
                    <FormDescription>Where new instances are created.</FormDescription>
                  </FormItem>
                )}
              />

              <SectionHeading title="Java & Performance" className="mt-7 mb-0" />
              <FormField
                control={form.control}
                name="javaRuntime"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Java runtime</FormLabel>
                    <Select value={field.value} onValueChange={field.onChange}>
                      <FormControl>
                        <SelectTrigger className="w-full">
                          <SelectValue />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        {RUNTIMES.map((r) => (
                          <SelectItem key={r} value={r}>
                            {r}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <FormDescription>
                      Auto-managed. Hestia downloads the right JDK per instance.
                    </FormDescription>
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="memory"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Default allocated memory — {memory} GB</FormLabel>
                    <FormControl>
                      <Slider
                        min={2}
                        max={24}
                        step={1}
                        value={[field.value]}
                        onValueChange={(v) => field.onChange(Array.isArray(v) ? v[0] : v)}
                      />
                    </FormControl>
                    <FormDescription>
                      Your system has 32 GB. Instances can override this individually.
                    </FormDescription>
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name="jvmArgs"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Default JVM arguments</FormLabel>
                    <FormControl>
                      <Input className="font-mono" {...field} />
                    </FormControl>
                  </FormItem>
                )}
              />

              <SectionHeading title="On launch" className="mt-7 mb-0" />
              {(
                [
                  ["keepOpen", "Keep the launcher open while a game runs"],
                  ["closeOnStart", "Close the launcher when a game starts"],
                  ["checkUpdates", "Check for mod updates on startup"],
                ] as const
              ).map(([name, label]) => (
                <FormField
                  key={name}
                  control={form.control}
                  name={name}
                  render={({ field }) => (
                    <FormItem className="flex-row items-center gap-2.5">
                      <FormControl>
                        <Checkbox checked={field.value} onCheckedChange={field.onChange} />
                      </FormControl>
                      <FormLabel className="font-normal text-fg-2">{label}</FormLabel>
                    </FormItem>
                  )}
                />
              ))}

              <div className="mt-1 flex items-center gap-2.5 border-t border-border-2 pt-4.5 text-xs text-fg-3">
                <img src={logoEmber} alt="" className="size-4.5 rounded-sm" />
                Hestia 0.0.1 · latest
              </div>
            </form>
          </Form>
        </div>
      </div>
    </>
  );
}

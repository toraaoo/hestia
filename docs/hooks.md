# Frontend hooks — the queries layer

The usage guide for `frontend/src/queries/`, the TanStack Query layer the
desktop UI is built on. It mirrors the typed API (`frontend/src/api/`) **1:1**
— every channel the daemon serves has a hook — so building a page is only
rendering: no `call()`, no event-bus wiring, no cache bookkeeping in
components. [contributing.md](contributing.md#add-a-desktop-api-method) has
the recipe for *extending* the layer; this page is how to *consume* it.

```
frontend/src/
├── api/          typed channel calls (no caching) — mirrors the Rust client SDK
└── queries/      this layer: factories + hooks + jobs + invalidation
    ├── client.ts        the QueryClient singleton + invalidate()
    ├── keys.ts          the hierarchical query-key factory
    ├── core.ts          mutation() — the plain-mutation factory helper
    ├── jobs.ts          the global job store + jobMutation()/useJobMutation
    ├── invalidation.ts  daemon topics → key prefixes (installed at bootstrap)
    ├── connection.ts    useConnection()
    ├── events.ts        useDaemonEvent(topic, handler)
    ├── logs.ts          log following shared by the per-domain log hooks
    └── <domain>.ts      server, instance, java, … — factories + 1:1 hooks
```

Import everything from the barrel:

```ts
import { useServer, useStartServer, serverQueries, keys } from '#/queries';
```

## The mental model

Each domain module exports three things:

- **`<domain>Queries`** — `queryOptions` factories. The single definition of
  a read: its key, its fetch, its staleness. Usable anywhere an options
  object is: `useQuery(serverQueries.detail(id))`, a router loader's
  `ensureQueryData`, `queryClient.fetchQuery`.
- **`<domain>Mutations`** — mutation-options factories. The single
  definition of a write: what it calls and which key prefixes it
  invalidates on settle (declared as data, not scattered in components).
- **hooks** — one per API function, each a one-liner over the factory:
  `useServers()`, `useStartServer(id)`, `useCreateServerBackup(id)`.

Rules that hold everywhere:

- **Per-entry hooks take the stable `id`** (from list data), never the
  display name. The wire resolves either, but a rename must not strand a
  cache key or a mutation key.
- **Errors are `HestiaError`** (`error.code` is the daemon's error-code
  vocabulary — `not_found`, `bad_request`, …). The type is registered
  globally, so `query.error`/`mutation.error` are typed without casts.
- **Freshness is automatic.** Mutations invalidate their own prefixes on
  settle, and the daemon-event feed (`invalidation.ts`) sweeps keys when the
  CLI, the tray, or the scheduler changes something. Components never call
  `invalidateQueries` themselves.

## Reads

```tsx
function ServerList() {
  const { data: servers, isPending, error } = useServers();
  if (isPending) return <Spinner />;
  if (error) return <p role="alert">{error.message}</p>;
  return servers.map((s) => <ServerCard key={s.id} id={s.id} />);
}
```

`useServer(id)` seeds from the list cache, so rendering a row of an
already-fetched list costs no extra call; the status query then keeps it
fresh. Instances have no status channel, so `useInstance(id)` selects the
entry out of the list query — same shape to the caller.

```tsx
const { data: server } = useServer(id);        // ServerInfo | undefined
const { data: instance } = useInstance(id);    // InstanceInfo | null | undefined
const { data: backups } = useServerBackups(id);
const { data: mods } = useServerContent(id, 'mod');
const { data: value } = useServerConfigValue(id, 'memory');   // string | null
```

Catalogue reads (`useServerFlavors`, `useServerVersions(flavor)`,
`useJavaReleases`, content browse) carry a longer `staleTime` — upstream
catalogues don't change mid-session.

## Loading skeletons

Pending reads render hand-drawn, theme-matched skeletons — ordinary
components over the same tokens (`bg-muted`, the app's square corners), no
capture step and no dependency. `Bone` (`components/skeleton.tsx`) is the
pulsing primitive; `CardGridSkeleton` mirrors a page's real grid classes so
bones land where cards will; a page-shaped composition lives beside any page
that needs more (`features/skins/skeleton.tsx`, the entry grid shared by
servers/instances/library in `features/entries/skeleton.tsx`). Two seams:

- A routed page passes `skeleton={<… />}` with its pending flag to `Page` —
  `<Page skeleton={<EntryGridSkeleton />} loading={isPending} …>` — and the
  body swaps for the skeleton while the header stays live.
- A surface outside `Page` (the sidebar account row) renders its bones
  inline while its query is pending.

## Writes

Every mutation hook returns a standard TanStack mutation: `mutate` /
`mutateAsync`, `isPending`, `error`, `data`. The variables are whatever the
verb needs *beyond* the entry already bound at hook time:

```tsx
const start = useStartServer(id);
const rename = useRenameServer(id);
const setConfig = useSetServerConfig(id);

start.mutate();                                  // no variables
rename.mutate('Cozy SMP');                       // the new name
setConfig.mutate({ key: 'memory', value: '4G' });
```

```tsx
<Button onClick={() => start.mutate()} disabled={start.isPending}>
  Start
</Button>
{start.error && <p role="alert">{start.error.message}</p>}
```

Invalidation happens on settle via the factory's `invalidates` — after
`start` settles, the server lists/details and process queries refetch on
their own. `mutate` accepts the usual per-call callbacks when a component
needs them (`onSuccess: (server) => navigate(…)`).

## Long-running jobs

Anything that streams progress events — server create/update, instance
launch, backups, content installs, java installs, downloads — is a **job
mutation**. Two guarantees:

1. **Every run lands in the global job store**, no matter which component
   fired it or whether that component is still mounted. An activity
   panel/toast surface subscribes with `useJobs()`; a per-entry busy
   indicator with `useEntryJobs(kind, id)`.
2. **`useJobMutation` adds the local view**: the same mutation result plus
   `progress` (the job's latest progress payload) and `job` (the store's
   record) for the run this call site started.

```tsx
function CreateServer() {
  const create = useCreateServer();

  const submit = (form: ServerCreateParams) =>
    create.mutate(form, {
      onSuccess: (server) => navigate({ to: `/servers/${server.id}` }),
    });

  if (create.isPending)
    return (
      <ProgressBar
        label={create.progress?.phase}          // 'resolving' | 'java' | …
        value={create.progress?.current}
        max={create.progress?.total}
      />
    );
  return <WizardForm onSubmit={submit} error={create.error} />;
}
```

```tsx
function ActivityPanel() {
  const jobs = useJobs();
  return jobs.map((job) => (
    <Row key={job.id}>
      <span>{job.label}</span>
      {job.status === 'running' && <Gauge progress={job.progress} />}
      {job.status === 'error' && <em>{job.error?.message}</em>}
      {job.status !== 'running' && (
        <button type="button" onClick={() => dismissJob(job.id)}>×</button>
      )}
    </Row>
  ));
}
```

A `Job` carries `kind` (`'server.create'`, `'backup.restore'`, …), an
optional `entry` (`{ kind: 'server' | 'instance', id }`), `status`
(`running | done | error`), the latest `progress`, and timestamps. Settled
jobs stay listed until `dismissJob(id)` / `clearSettledJobs()` (the store
caps how many settled jobs it keeps).

A component that fires a job but renders no inline progress can use plain
`useMutation(serverMutations.backup.create(id))` — the run is tracked
globally either way; `useJobMutation` only adds the local `progress` view.

## Live data

**Connection state** — the shell's watcher, as a hook. On reconnect the
layer invalidates every query itself; the banner is all the UI owes:

```tsx
const connection = useConnection();   // 'connected' | 'disconnected'
if (connection === 'disconnected') return <Banner>Daemon unreachable…</Banner>;
```

**Logs** — the fetched tail plus `process.output` events accumulated on
top. `lines` is the merged view; the rest is the underlying query result:

```tsx
function Console({ id }: { id: string }) {
  const logs = useServerLogs(id, { tail: 200, follow: true });
  const command = useServerCommand(id);
  return (
    <>
      <LogView lines={logs.lines} />
      <Input onSubmit={(line) => command.mutate(line)} />
    </>
  );
}
```

`useInstanceLogs(id, { session, follow })` follows one named session or
every session of the instance; `useProcessLogs(processId, …)` is the raw
per-process form. While following, the tail query stops refetching (the
stream is the freshness) and the live buffer is capped by `limit`
(default 1000).

**Raw events** — for the rare component that needs a daemon topic directly
(the payload shapes mirror `crates/proto`'s `Topic` structs):

```tsx
useDaemonEvent<ProcessExit>('process.exit', (exit) => {
  if (!exit.success) toast.error(`${exit.id} crashed`);
});
```

## Route loaders

Factories plug straight into TanStack Router. The root route context
carries the `queryClient`:

```tsx
export const Route = createFileRoute('/servers/')({
  loader: ({ context }) =>
    context.queryClient.ensureQueryData(serverQueries.list()),
  component: ServersPage,   // useServers() renders instantly from the cache
});
```

## Error handling

```tsx
const { error } = useServer(id);
if (error && isNotFound(error)) return <NotFound />;   // from '#/api'
```

`tryCall`-backed reads (`useConfigValue`, `useServerConfigValue`, …) already
surface a missing value as `null` data rather than an error. Queries and
mutations don't retry (the daemon is a local socket — failures aren't
transient network blips), so an error is real the first time you see it.

## Hook inventory

The 1:1 audit: every hook, its wire surface, and its variables. *(job)*
marks a job mutation — pair with `useJobMutation` semantics above.

### Cross-cutting

| Hook | What |
|---|---|
| `useConnection()` | daemon connection state |
| `useDaemonEvent(topic, handler)` | one daemon topic, for the component's lifetime |
| `useJobs()` / `useJob(id)` / `useEntryJobs(kind, id)` | the global job store |
| `dismissJob(id)` / `clearSettledJobs()` / `getJobs()` | store maintenance (not hooks) |

### Servers

| Hook | Channel(s) | Variables |
|---|---|---|
| `useServers()` | `server.list` | — |
| `useServer(id)` | `server.status` (seeded from list) | — |
| `useServerFlavors()` / `useServerVersions(flavor)` / `useServerProfile(params)` | `server.flavors\|versions\|resolve` | — |
| `useServerLogs(id, { tail, follow, limit })` | `server.logs` + `process.output` | — |
| `useServerConfig(id)` / `useServerConfigValue(id, key)` | `server.config.list\|get` | — |
| `useServerBackups(id)` | `server.backup.list` | — |
| `useServerContent(id, kind)` | `server.content.list` | — |
| `useCreateServer()` *(job)* | `server.create` | `ServerCreateParams` |
| `useUpdateServer(id)` *(job)* | `server.update` | `{ version, loader_version?, allow_downgrade? }` |
| `useRenameServer(id)` | `server.rename` | `name: string` |
| `useRemoveServer(id)` | `server.remove` | — |
| `useStartServer(id)` / `useStopServer(id)` | `server.start\|stop` | — |
| `useServerCommand(id)` | `server.command` | `line: string` |
| `useSetServerConfig(id)` | `server.config.set` | `{ key, value }` |
| `useCreateServerBackup(id)` *(job)* | `server.backup.create` | — |
| `useRestoreServerBackup(id)` *(job)* | `server.backup.restore` | `backupId: string` |
| `useRemoveServerBackup(id)` | `server.backup.remove` | `backupId: string` |
| `useAddServerContent(id)` *(job)* | `server.content.add` | `ContentAddSpec` |
| `useRemoveServerContent(id)` | `server.content.remove` | `{ kind, item, worlds? }` |
| `useUpdateServerContent(id)` *(job)* | `server.content.update` | `{ kind, item? }` |

### Instances

| Hook | Channel(s) | Variables |
|---|---|---|
| `useInstances()` | `instance.list` | — |
| `useInstance(id)` | selected from `instance.list` | — |
| `useInstanceFlavors()` / `useInstanceVersions(flavor)` / `useInstanceProfile(params)` | `instance.flavors\|versions\|resolve` | — |
| `useInstanceWorlds(id)` | `instance.worlds` | — |
| `useInstanceLogs(id, { session, tail, follow, limit })` | `instance.logs` + `process.output` | — |
| `useInstanceConfig(id)` / `useInstanceConfigValue(id, key)` | `instance.config.list\|get` | — |
| `useInstanceContent(id, kind)` | `instance.content.list` | — |
| `useInstanceProfiles(id)` | `instance.profile.list` | — |
| `useCreateInstance()` | `instance.create` | `InstanceCreateParams` |
| `useUpdateInstance(id)` | `instance.update` | `{ version, loader_version?, allow_downgrade? }` |
| `useRenameInstance(id)` | `instance.rename` | `name: string` |
| `useRemoveInstance(id)` | `instance.remove` | — |
| `useLaunchInstanceAny()` *(job)* | `instance.launch` | `id: string` |
| `useStopInstance(id)` | `instance.stop` | `{ session? }` |
| `useSetInstanceConfig(id)` | `instance.config.set` | `{ key, value }` |
| `useAddInstanceContent(id)` *(job)* | `instance.content.add` | `ContentAddSpec` |
| `useRemoveInstanceContent(id)` | `instance.content.remove` | `{ kind, item, worlds? }` |
| `useUpdateInstanceContent(id)` *(job)* | `instance.content.update` | `{ kind, item? }` |
| `useCreateInstanceProfile(id)` | `instance.profile.create` | `{ name, seedFromPool? }` |
| `useRemoveInstanceProfile(id)` | `instance.profile.remove` | `name: string` |
| `useRenameInstanceProfile(id)` | `instance.profile.rename` | `{ name, newName }` |
| `useUseInstanceProfile(id)` | `instance.profile.use` | `name: string` (empty clears) |
| `useEditInstanceProfile(id)` | `instance.profile.edit` | `{ name, add?, remove? }` |
| `useCaptureInstanceProfile(id)` | `instance.profile.capture` | `name: string` |
| `useReleaseInstanceProfile(id)` | `instance.profile.release` | `name: string` |
| `useApplyInstanceProfile(id)` *(job)* | `instance.profile.apply` | `profile: string` |

### Global profiles

| Hook | Channel | Variables |
|---|---|---|
| `useGlobalProfiles()` | `profile.list` | — |
| `useCreateGlobalProfile()` | `profile.create` | `name: string` |
| `useRemoveGlobalProfile()` | `profile.remove` | `name: string` |
| `useEditGlobalProfile()` | `profile.edit` | `{ name, source?, add?, remove? }` |

### Content browse

| Hook | Channel | Variables |
|---|---|---|
| `useContentSources()` | `content.sources` | — |
| `useContentSearch(query)` | `content.search` | — |
| `useContentProject(project, source?)` | `content.project` | — |
| `useContentVersions(query)` | `content.versions` | — |
| `useResolvedModpack(versionId, source?)` | `content.modpack.resolve` | — (heavy — mount deliberately) |

### Everything else

| Hook | Channel | Variables |
|---|---|---|
| `useAppInfo()` / `usePing()` | `app.info` / `health.ping` | — |
| `useDaemonStatus()` | `daemon.status` | — |
| `useStopDaemon()` | `daemon.stop` | `{ stopProcesses }` |
| `useConfig()` / `useConfigValue(key)` | `config.list\|get` | — |
| `useSetConfig()` | `config.set` | `{ key, value }` |
| `useCacheInfo()` / `useCacheEntries()` | `cache.info\|list` | — |
| `useClearCache()` | `cache.clear` | — |
| `useAccounts()` | `account.list` | — |
| `useBeginLogin()` | `account.login.begin` | `'sisu' \| 'device_code'` |
| `useCompleteLogin()` | `account.login.complete` | `{ id, code? }` |
| `useSwitchAccount()` / `useRemoveAccount()` | `account.switch\|remove` | `account: string` |
| `useJavaReleases()` / `useJavaRuntimes()` | `java.releases\|list` | — |
| `useInstallJava()` *(job)* | `java.install` | `{ major, force? }` |
| `useUninstallJava()` | `java.uninstall` | `major: number` |
| `useProcesses()` / `useProcess(id)` | `process.list\|status` | — |
| `useProcessLogs(id, { tail, follow, limit })` | `process.logs` + `process.output` | — |
| `useStartProcess()` / `useStopProcess()` | `process.start\|stop` | `ProcessSpec` / `id: string` |
| `useSkins(account?)` | `skin.list` | — |
| `useAddSkin()` | `skin.add` | `{ account?, name?, variant, data }` |
| `useEquipSkin()` | `skin.equip` | `{ key, account? }` |
| `useResetSkin()` | `skin.reset` | `{ account? }?` |
| `useRemoveSkin()` | `skin.remove` | `key: string` |
| `useEquipCape()` / `useClearCape()` | `cape.equip\|clear` | `{ cape, account? }` / `{ account? }?` |
| `useSyncConfig()` | `sync.get` | — |
| `useSyncStatus()` | `sync.status` | — |
| `useSetSyncTargets()` | `sync.set` | `SyncTargets` |
| `useAdoptInstanceSync(id)` | `instance.sync.adopt` | `targets?: string[]` |
| `useStartDownload()` *(job)* | `download.start` | `Omit<DownloadSpec, 'id'>` |

## Extending the layer

A new channel is one factory entry plus a one-line hook in the domain's
`queries/<domain>.ts` (and a row here) — the full recipe, including the
`mutation()`/`jobMutation()` helpers and the invalidation map, is in
[contributing.md](contributing.md#add-a-desktop-api-method).

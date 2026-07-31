# Demo tapes

The CLI GIFs in [`../demo/`](../demo) are recorded with
[VHS](https://github.com/charmbracelet/vhs). One tape per GIF; `common.tape`
holds the shared terminal size, font and palette.

```bash
cargo build -p cli -p daemon
./record.sh                 # every tape
./record.sh content.tape    # just one
```

`record.sh` restores `~/.hestia-demo` from `~/.hestia-demo-base` before every
take, so a recording always starts from the same state and never touches a real
data home.

## Building the base

The base is an ordinary data home with a signed-in account, a Java runtime, and
one instance — everything a tape needs but doesn't demonstrate. Build it once:

```bash
export HESTIA_HOME=~/.hestia-demo
hestia daemon start
hestia account login
hestia java install 21
hestia instance create fabric 1.21.4 --name Skyblock
hestia daemon stop

cp -r ~/.hestia-demo ~/.hestia-demo-base
```

Copying an existing `cache/` and `meta/` into the base makes a take much faster,
since nothing has to be re-downloaded mid-recording.

**The account is real, so check every take before committing it.** No tape runs
a command that prints the profile name, but the base holds live tokens — never
publish the base itself, and never add `account` commands to a tape.

#!/usr/bin/env bash
set -euo pipefail

rid="${1:-}"
version="${2:-}"

if [[ -z "$rid" || -z "$version" ]]; then
  echo "usage: $0 <rid> <version>" >&2
  exit 2
fi

case "$rid" in
  win-x64|linux-x64) ;;
  *) echo "unsupported rid: $rid" >&2; exit 2 ;;
esac

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stage_dir="$repo_root/artifacts/stage/$rid"

rm -rf "$stage_dir"
mkdir -p "$stage_dir"

# Ensure restore includes the RID-specific asset targets.
dotnet restore "$repo_root/Hestia.sln" -r "$rid"

publish_project() {
  local project_path="$1"
  local out_dir="$2"

  dotnet publish "$project_path" \
    -c Release \
    -r "$rid" \
    --no-restore \
    --self-contained true \
    -p:PublishSingleFile=true \
    -p:IncludeNativeLibrariesForSelfExtract=true \
    -p:PublishTrimmed=false \
    -p:DebugType=None \
    -p:DebugSymbols=false \
    -p:Version="$version" \
    -p:InformationalVersion="$version" \
    -o "$out_dir"
}

tui_out="$stage_dir/tui"
desktop_out="$stage_dir/desktop"

publish_project "$repo_root/src/Hestia.Tui/Hestia.Tui.csproj" "$tui_out"
publish_project "$repo_root/src/Hestia.Desktop/Hestia.Desktop.csproj" "$desktop_out"

# Flatten into the stage root.
# If both publish outputs ever contain the same relative path, fail fast.
copy_tree_no_conflicts() {
  local src_dir="$1"
  local dest_dir="$2"

  while IFS= read -r -d '' file; do
    local rel="${file#"$src_dir/"}"
    if [[ -e "$dest_dir/$rel" ]]; then
      echo "staging collision: $rel" >&2
      exit 3
    fi
  done < <(find "$src_dir" -type f -print0)

  cp -a "$src_dir/." "$dest_dir/"
}

copy_tree_no_conflicts "$tui_out" "$stage_dir"
copy_tree_no_conflicts "$desktop_out" "$stage_dir"

rm -rf "$tui_out" "$desktop_out"

# Strip debug symbols from release artifacts.
find "$stage_dir" -type f -name '*.pdb' -delete

# Ensure the installed command is `hestia`.
if [[ "$rid" == "linux-x64" ]]; then
  chmod +x "$stage_dir/hestia"
fi

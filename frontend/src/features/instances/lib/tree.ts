/**
 * Turning the daemon's flat, path-sorted archive listing into the tree the
 * export dialog renders, and turning the boxes back into the exclusion paths
 * the daemon takes.
 *
 * Kept out of the component so the two conversions can be tested directly:
 * they are the part where getting it wrong silently ships (or drops) somebody's
 * worlds.
 */
import type { ArchiveEntry } from '@/api';

export interface TreeNode extends ArchiveEntry {
  children: TreeNode[];
}

/**
 * Build the tree. The listing is sorted by path, so every node's parent is
 * already present by the time it is reached — a node whose parent is missing
 * (a listing truncated mid-branch) is attached at the root rather than
 * dropped, since a file nobody can see is a file nobody can exclude.
 */
export function buildTree(entries: ArchiveEntry[]): TreeNode[] {
  const nodes = new Map<string, TreeNode>();
  const roots: TreeNode[] = [];

  for (const entry of entries) {
    const node: TreeNode = { ...entry, children: [] };
    nodes.set(entry.path, node);
    const cut = entry.path.lastIndexOf('/');
    const parent = cut > 0 ? nodes.get(entry.path.slice(0, cut)) : undefined;
    if (parent) parent.children.push(node);
    else roots.push(node);
  }
  return roots;
}

/**
 * The exclusions to send: only the **topmost** unchecked paths. Everything
 * under an excluded directory is already out, and listing it again would just
 * be noise on the wire.
 */
export function excludedRoots(excluded: Set<string>): string[] {
  const paths = [...excluded].sort();
  return paths.filter(
    (path) => !paths.some((other) => path.startsWith(`${other}/`)),
  );
}

/** What the archive would weigh with the current boxes. */
export function selectedBytes(tree: TreeNode[], excluded: Set<string>): number {
  let total = 0;
  const walk = (nodes: TreeNode[], parentOut: boolean) => {
    for (const node of nodes) {
      const out = parentOut || excluded.has(node.path);
      // A directory's size already covers everything beneath it, including the
      // parts too deep to be listed — so only descend when something inside is
      // excluded, and count the whole node otherwise.
      const partial =
        !out && node.children.some((child) => hasExclusion(child, excluded));
      if (partial) walk(node.children, false);
      else if (!out) total += node.sizeBytes;
    }
  };
  walk(tree, false);
  return total;
}

function hasExclusion(node: TreeNode, excluded: Set<string>): boolean {
  if (excluded.has(node.path)) return true;
  return node.children.some((child) => hasExclusion(child, excluded));
}

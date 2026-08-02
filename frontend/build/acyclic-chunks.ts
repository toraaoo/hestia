import type { Plugin } from 'vite';

/** Fail the build on a cyclic chunk graph, which loads as a blank window. */
export function acyclicChunks(): Plugin {
  return {
    name: 'hestia:acyclic-chunks',
    apply: 'build',
    generateBundle(_options, bundle) {
      const imports = new Map<string, string[]>();
      for (const [name, output] of Object.entries(bundle)) {
        if (output.type === 'chunk') {
          imports.set(name, output.imports);
        }
      }

      const visited = new Set<string>();
      const stack: string[] = [];
      const onStack = new Set<string>();

      const walk = (name: string): string[] | null => {
        if (onStack.has(name)) {
          return [...stack.slice(stack.indexOf(name)), name];
        }
        if (visited.has(name)) return null;
        visited.add(name);
        stack.push(name);
        onStack.add(name);
        for (const next of imports.get(name) ?? []) {
          const cycle = walk(next);
          if (cycle) return cycle;
        }
        stack.pop();
        onStack.delete(name);
        return null;
      };

      for (const name of imports.keys()) {
        const cycle = walk(name);
        if (cycle) {
          this.error(
            `chunks import each other in a cycle, which loads as a blank window:\n  ${cycle.join('\n  → ')}\nAdjust build.rolldownOptions.output.codeSplitting.groups so the graph is a DAG.`,
          );
        }
      }
    },
  };
}

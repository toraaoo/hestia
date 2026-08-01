import { installInternals } from '@/mock/internals';

let installed = false;

export function installFakeDaemon(): void {
  if (installed) return;
  installed = true;
  installInternals();
}

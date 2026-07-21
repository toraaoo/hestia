/**
 * Texture helpers: the shared image-loading cache, the flat cape-front blit
 * for the cape picker, and file reading for uploads. Full-body card renders
 * are posed snapshots in `skin-render` — cards never blit skin layouts here.
 */

const textures = new Map<string, Promise<HTMLImageElement>>();

export function loadTexture(src: string): Promise<HTMLImageElement> {
  let pending = textures.get(src);
  if (!pending) {
    pending = new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error(`failed to load texture: ${src}`));
      img.src = src;
    });
    textures.set(src, pending);
  }
  return pending;
}

/** The cape's front face is the 10x16 region at (1,1) of a 64x32 texture. */
export const CAPE_W = 10;
export const CAPE_H = 16;

export function drawCapeFront(
  canvas: HTMLCanvasElement,
  img: HTMLImageElement,
) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, CAPE_W, CAPE_H);
  ctx.drawImage(img, 1, 1, CAPE_W, CAPE_H, 0, 0, CAPE_W, CAPE_H);
}

/** Reads a skin file into a data URL for previewing before it is saved. */
export function readTextureFile(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(new Error('failed to read file'));
    reader.readAsDataURL(file);
  });
}

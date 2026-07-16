/**
 * Canvas helpers for flat skin renders: full-body front views for library
 * cards and cape fronts for the cape picker. The heavyweight animated model
 * (skinview3d) is only mounted for the main preview; cards stay cheap 2D
 * blits so a grid of dozens never spins up dozens of WebGL contexts.
 */

import type { SkinVariant } from '@/features/skins/mock';

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

/** The front-view body composite is 16x32 texture pixels. */
export const BODY_W = 16;
export const BODY_H = 32;

interface Part {
  sx: number;
  sy: number;
  w: number;
  h: number;
  dx: number;
  dy: number;
  mirror?: boolean;
}

/**
 * Front faces in the modern 64x64 layout. Arm width narrows to 3 for the
 * slim variant; overlay parts sit on the same destination as their base.
 */
function parts(variant: SkinVariant, legacy: boolean): Part[] {
  const aw = variant === 'slim' ? 3 : 4;
  const base: Part[] = [
    { sx: 20, sy: 20, w: 8, h: 12, dx: 4, dy: 8 },
    { sx: 44, sy: 20, w: aw, h: 12, dx: 4 - aw, dy: 8 },
    { sx: 4, sy: 20, w: 4, h: 12, dx: 4, dy: 20 },
  ];
  if (legacy) {
    // Pre-1.8 64x32 skins mirror the right limbs for the left side and only
    // carry the hat overlay.
    base.push(
      { sx: 44, sy: 20, w: aw, h: 12, dx: 12, dy: 8, mirror: true },
      { sx: 4, sy: 20, w: 4, h: 12, dx: 8, dy: 20, mirror: true },
      { sx: 8, sy: 8, w: 8, h: 8, dx: 4, dy: 0 },
      { sx: 40, sy: 8, w: 8, h: 8, dx: 4, dy: 0 },
    );
    return base;
  }
  base.push(
    { sx: 36, sy: 52, w: aw, h: 12, dx: 12, dy: 8 },
    { sx: 20, sy: 52, w: 4, h: 12, dx: 8, dy: 20 },
    // Overlays, innermost first: jacket, sleeves, pants, then head + hat on
    // top so the hat brim never sits under body pixels.
    { sx: 20, sy: 36, w: 8, h: 12, dx: 4, dy: 8 },
    { sx: 44, sy: 36, w: aw, h: 12, dx: 4 - aw, dy: 8 },
    { sx: 52, sy: 52, w: aw, h: 12, dx: 12, dy: 8 },
    { sx: 4, sy: 36, w: 4, h: 12, dx: 4, dy: 20 },
    { sx: 4, sy: 52, w: 4, h: 12, dx: 8, dy: 20 },
    { sx: 8, sy: 8, w: 8, h: 8, dx: 4, dy: 0 },
    { sx: 40, sy: 8, w: 8, h: 8, dx: 4, dy: 0 },
  );
  return base;
}

export function drawSkinFront(
  canvas: HTMLCanvasElement,
  img: HTMLImageElement,
  variant: SkinVariant,
) {
  const ctx = canvas.getContext('2d');
  if (!ctx) return;
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, BODY_W, BODY_H);
  const legacy = img.height < 64;
  for (const p of parts(variant, legacy)) {
    if (p.mirror) {
      ctx.save();
      ctx.translate(p.dx + p.w, p.dy);
      ctx.scale(-1, 1);
      ctx.drawImage(img, p.sx, p.sy, p.w, p.h, 0, 0, p.w, p.h);
      ctx.restore();
    } else {
      ctx.drawImage(img, p.sx, p.sy, p.w, p.h, p.dx, p.dy, p.w, p.h);
    }
  }
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

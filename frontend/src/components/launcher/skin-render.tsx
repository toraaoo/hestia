import { useEffect, useRef } from 'react';
import { SkinViewer, WalkingAnimation } from 'skinview3d';

import type { SkinVariant } from '@/lib/mock';
import {
  BODY_H,
  BODY_W,
  CAPE_H,
  CAPE_W,
  drawCapeFront,
  drawSkinFront,
  loadTexture,
} from '@/lib/skin';
import { cn } from '@/lib/utils';

/** Flat front view of a full skin — the cheap render for library cards. */
export function SkinBody({
  texture,
  variant,
  className,
}: {
  texture: string;
  variant: SkinVariant;
  className?: string;
}) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let live = true;
    loadTexture(texture)
      .then((img) => {
        if (live && ref.current) drawSkinFront(ref.current, img, variant);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [texture, variant]);

  return (
    <canvas
      ref={ref}
      width={BODY_W}
      height={BODY_H}
      aria-hidden
      className={cn('[image-rendering:pixelated]', className)}
    />
  );
}

/** Flat front face of a cape texture, for the cape picker. */
export function CapeFront({
  texture,
  className,
}: {
  texture: string;
  className?: string;
}) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    let live = true;
    loadTexture(texture)
      .then((img) => {
        if (live && ref.current) drawCapeFront(ref.current, img);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [texture]);

  return (
    <canvas
      ref={ref}
      width={CAPE_W}
      height={CAPE_H}
      aria-hidden
      className={cn('[image-rendering:pixelated]', className)}
    />
  );
}

/**
 * The animated 3D player model (skinview3d). One instance per surface — the
 * main preview panel and the edit modal — never per card.
 */
export function SkinModel({
  texture,
  capeTexture,
  variant,
  width,
  height,
  className,
}: {
  texture: string;
  capeTexture?: string;
  variant: SkinVariant;
  width: number;
  height: number;
  className?: string;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const viewerRef = useRef<SkinViewer | null>(null);

  useEffect(() => {
    if (!canvasRef.current) return;
    const viewer = new SkinViewer({
      canvas: canvasRef.current,
      zoom: 0.85,
    });
    viewer.controls.enableZoom = false;
    viewer.controls.enablePan = false;
    viewer.animation = new WalkingAnimation();
    viewer.animation.speed = 0.55;
    viewer.playerObject.rotation.y = Math.PI / 9;
    viewerRef.current = viewer;
    return () => {
      viewerRef.current = null;
      viewer.dispose();
    };
    // The viewer mounts once; size and textures are applied by the effects
    // below (which also run on mount) without rebuilding the WebGL context.
  }, []);

  useEffect(() => {
    viewerRef.current?.setSize(width, height);
  }, [width, height]);

  useEffect(() => {
    viewerRef.current
      ?.loadSkin(texture, {
        model: variant === 'slim' ? 'slim' : 'default',
      })
      .catch(() => {});
  }, [texture, variant]);

  useEffect(() => {
    const viewer = viewerRef.current;
    if (!viewer) return;
    if (capeTexture) viewer.loadCape(capeTexture).catch(() => {});
    else viewer.resetCape();
  }, [capeTexture]);

  return <canvas ref={canvasRef} className={className} />;
}

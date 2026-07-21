import { useEffect, useRef, useState } from 'react';
import { SkinViewer, WalkingAnimation } from 'skinview3d';

import type { SkinVariant } from '@/api';
import {
  CAPE_H,
  CAPE_W,
  drawCapeFront,
  loadTexture,
} from '@/features/skins/lib/texture';
import { cn } from '@/lib/utils';

const POSE_WIDTH = 160;
const POSE_HEIGHT = 256;

let poseViewer: SkinViewer | null = null;
let poseQueue: Promise<unknown> = Promise.resolve();
const poseSnapshots = new Map<string, Promise<string>>();

function posedViewer(): SkinViewer {
  if (!poseViewer) {
    poseViewer = new SkinViewer({
      canvas: document.createElement('canvas'),
      width: POSE_WIDTH,
      height: POSE_HEIGHT,
      zoom: 0.9,
      renderPaused: true,
    });
    poseViewer.playerObject.rotation.y = Math.PI / 9;
    const parts = poseViewer.playerObject.skin;
    parts.rightArm.rotation.x = 0.25;
    parts.leftArm.rotation.x = -0.25;
    parts.rightLeg.rotation.x = -0.2;
    parts.leftLeg.rotation.x = 0.2;
  }
  return poseViewer;
}

// One shared paused viewer: a card grid must never hold WebGL contexts apiece.
function poseSnapshot(texture: string, variant: SkinVariant): Promise<string> {
  const key = `${variant}|${texture}`;
  let pending = poseSnapshots.get(key);
  if (!pending) {
    const run = poseQueue.then(async () => {
      const viewer = posedViewer();
      await viewer.loadSkin(texture, {
        model: variant === 'slim' ? 'slim' : 'default',
      });
      viewer.render();
      // Same-task readback: the unpreserved buffer is gone after compositing.
      return viewer.canvas.toDataURL();
    });
    poseQueue = run.catch(() => undefined);
    poseSnapshots.set(key, run);
    run.catch(() => poseSnapshots.delete(key));
    pending = run;
  }
  return pending;
}

/** A static posed render of a full skin — the card-grid view. */
export function SkinPose({
  texture,
  variant,
  className,
}: {
  texture: string;
  variant: SkinVariant;
  className?: string;
}) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setSrc(null);
    poseSnapshot(texture, variant)
      .then((url) => {
        if (live) setSrc(url);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [texture, variant]);

  if (!src) return <div aria-hidden className={className} />;
  return <img src={src} alt="" className={cn('object-contain', className)} />;
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

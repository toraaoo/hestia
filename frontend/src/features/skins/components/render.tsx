import { useEffect, useRef, useState } from 'react';

import type { SkinVariant } from '@/api';
import {
  CAPE_H,
  CAPE_W,
  drawCapeFront,
  loadTexture,
} from '@/features/skins/lib/texture';
import type { SkinPreview } from '@/features/skins/lib/webgl/preview';
import { cn } from '@/lib/utils';
import { m } from '@/paraglide/messages.js';

const previewModule = () => import('@/features/skins/lib/webgl/preview');
const thumbnailModule = () => import('@/features/skins/lib/webgl/thumbnails');

/** A static posed render of a full skin — the card-grid view. */
export function SkinPose({
  texture,
  variant,
  capeTexture,
  className,
}: {
  texture: string;
  variant: SkinVariant;
  capeTexture?: string;
  className?: string;
}) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    setSrc(null);
    thumbnailModule()
      .then(({ thumbnail }) => thumbnail(variant, texture, capeTexture))
      .then((url) => {
        if (live) setSrc(url);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [texture, variant, capeTexture]);

  if (!src) return <div aria-hidden className={className} />;
  return (
    <img
      src={src}
      alt=""
      className={cn('object-cover object-top', className)}
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
 * The animated 3D player model. One instance per surface — the main preview
 * panel and the edit modal — never per card; each holds a WebGL context.
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
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const previewRef = useRef<SkinPreview | null>(null);
  const latest = useRef({ variant, texture, capeTexture });
  latest.current = { variant, texture, capeTexture };

  useEffect(() => {
    if (!containerRef.current || !canvasRef.current) return;
    let disposed = false;

    void previewModule()
      .then(({ SkinPreview }) => {
        if (disposed || !containerRef.current || !canvasRef.current) return;
        previewRef.current = new SkinPreview({
          canvas: canvasRef.current,
          container: containerRef.current,
          variant: latest.current.variant,
          texture: latest.current.texture,
          cape: latest.current.capeTexture,
        });
      })
      .catch(() => {});

    return () => {
      disposed = true;
      const preview = previewRef.current;
      previewRef.current = null;
      preview?.dispose();
    };
  }, []);

  useEffect(() => {
    void previewRef.current?.load(variant, texture, capeTexture);
  }, [variant, texture, capeTexture]);

  return (
    <div
      ref={containerRef}
      style={{ width, height }}
      className={cn('relative select-none', className)}
    >
      <canvas
        ref={canvasRef}
        className="size-full cursor-grab touch-none active:cursor-grabbing"
        onPointerDown={(e) =>
          previewRef.current?.pointerDown(e.nativeEvent, e.currentTarget)
        }
        onPointerMove={(e) => previewRef.current?.pointerMove(e.nativeEvent)}
        onPointerUp={(e) =>
          previewRef.current?.pointerUp(e.nativeEvent, e.currentTarget)
        }
        onPointerLeave={(e) =>
          previewRef.current?.pointerUp(e.nativeEvent, e.currentTarget)
        }
        onPointerCancel={(e) =>
          previewRef.current?.pointerUp(e.nativeEvent, e.currentTarget)
        }
        onClick={() => previewRef.current?.click()}
      />
      <span className="pointer-events-none absolute inset-x-0 bottom-2 text-center text-[11px] text-muted-foreground">
        {m['skin.drag_to_rotate']()}
      </span>
    </div>
  );
}

/**
 * Card thumbnails: one offscreen renderer draws every skin in turn, and the
 * results are cached in IndexedDB so a reload costs no WebGL work.
 */
import * as THREE from 'three';

import type { SkinVariant } from '@/api';
import { applyCapeTexture, setupSkinModel } from './skin-rendering';

// A full-body figure, angled off the front. The card crops it to a bust by
// covering with the top anchored, so the framing here is deliberately taller
// than any card that shows it.
const WIDTH = 360;
const HEIGHT = 504;
const FOV = 20;
const CAMERA_POSITION: [number, number, number] = [-1.3, 1, 6.3];
const GROUP_POSITION: [number, number, number] = [0, 0.3, 1.95];
const GROUP_SCALE = 0.8;

const DB_NAME = 'skin-previews';
const DB_VERSION = 1;
const STORE = 'previews';
/** Bump to invalidate every cached thumbnail after a rendering change. */
const RENDER_VERSION = 'v3';

class ThumbnailRenderer {
  private renderer: THREE.WebGLRenderer | null = null;
  private scene: THREE.Scene | null = null;
  private camera: THREE.PerspectiveCamera | null = null;
  private group: THREE.Group | null = null;

  private init(): void {
    if (this.renderer) return;

    const canvas = document.createElement('canvas');
    canvas.width = WIDTH;
    canvas.height = HEIGHT;

    this.renderer = new THREE.WebGLRenderer({
      canvas,
      antialias: true,
      alpha: true,
      preserveDrawingBuffer: true,
    });
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.toneMapping = THREE.NoToneMapping;
    this.renderer.toneMappingExposure = 10;
    this.renderer.setClearColor(0x000000, 0);
    this.renderer.setSize(WIDTH, HEIGHT);

    this.scene = new THREE.Scene();
    this.camera = new THREE.PerspectiveCamera(FOV, WIDTH / HEIGHT, 0.4, 1000);

    const light = new THREE.DirectionalLight(0xffffff, 1.2);
    light.position.set(2, 4, 3);
    this.scene.add(new THREE.AmbientLight(0xffffff, 2));
    this.scene.add(light);
  }

  async render(
    variant: SkinVariant,
    texture: string,
    cape?: string,
  ): Promise<Blob> {
    this.init();
    if (!this.renderer || !this.scene || !this.camera) {
      throw new Error('renderer unavailable');
    }

    this.clear();
    const model = await setupSkinModel(variant, texture, cape);
    if (!cape) applyCapeTexture(model, null);

    const group = new THREE.Group();
    group.add(model);
    group.position.set(...GROUP_POSITION);
    group.scale.setScalar(GROUP_SCALE);
    this.scene.add(group);
    this.group = group;

    const head = model.getObjectByName('Head');
    if (!head) throw new Error('model has no Head node');
    const headPosition = new THREE.Vector3();
    head.getWorldPosition(headPosition);

    this.camera.position.set(...CAMERA_POSITION);
    this.camera.lookAt(headPosition.x, headPosition.y - 0.3, headPosition.z);
    this.renderer.render(this.scene, this.camera);

    return new Promise<Blob>((resolve, reject) => {
      this.renderer?.domElement.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error('no blob'))),
        'image/webp',
        0.9,
      );
    });
  }

  private clear(): void {
    if (!this.scene || !this.group) return;
    this.scene.remove(this.group);
    this.group.clear();
    this.group = null;
  }
}

function openDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(STORE)) db.createObjectStore(STORE);
    };
  });
}

let database: Promise<IDBDatabase> | null = null;

function db(): Promise<IDBDatabase> {
  database ??= openDatabase();
  return database;
}

async function cached(key: string): Promise<Blob | null> {
  try {
    const store = (await db())
      .transaction([STORE], 'readonly')
      .objectStore(STORE);
    return await new Promise<Blob | null>((resolve, reject) => {
      const request = store.get(key);
      request.onsuccess = () =>
        resolve((request.result as { blob: Blob } | undefined)?.blob ?? null);
      request.onerror = () => reject(request.error);
    });
  } catch {
    return null;
  }
}

async function store(key: string, blob: Blob): Promise<void> {
  try {
    const objects = (await db())
      .transaction([STORE], 'readwrite')
      .objectStore(STORE);
    objects.put({ blob, timestamp: Date.now() }, key);
  } catch {
    // A thumbnail that cannot be cached is re-rendered next time; not fatal.
  }
}

const renderer = new ThumbnailRenderer();
const urls = new Map<string, Promise<string>>();
// Renders serialize through the one WebGL context.
let queue: Promise<unknown> = Promise.resolve();

function key(variant: SkinVariant, texture: string, cape?: string): string {
  return `${RENDER_VERSION}|${variant}|${texture}|${cape ?? 'no-cape'}`;
}

/** An object URL for the skin's posed thumbnail, rendered once per key. */
export function thumbnail(
  variant: SkinVariant,
  texture: string,
  cape?: string,
): Promise<string> {
  const id = key(variant, texture, cape);
  const existing = urls.get(id);
  if (existing) return existing;

  const url = (async () => {
    const hit = await cached(id);
    if (hit) return URL.createObjectURL(hit);

    const render = queue.then(() => renderer.render(variant, texture, cape));
    queue = render.catch(() => undefined);
    const blob = await render;
    void store(id, blob);
    return URL.createObjectURL(blob);
  })();

  urls.set(id, url);
  void url.catch(() => urls.delete(id));
  return url;
}

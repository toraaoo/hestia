/**
 * The preview's two shaders: the radial floor spotlight, and the damage flash
 * injected into each skin material.
 */
import * as THREE from 'three';

type FlashMaterial = THREE.MeshStandardMaterial & {
  userData: {
    flashShader?: THREE.WebGLProgramParametersWithUniforms;
    flashInstalled?: boolean;
  };
};

const FLASH_COLOR = new THREE.Color(0xbd2f2f);
const FLASH_CACHE_KEY = 'skin-preview-damage-flash';

export function spotlightMaterial(): THREE.ShaderMaterial {
  return new THREE.ShaderMaterial({
    uniforms: {
      innerColor: { value: new THREE.Color(0x000000) },
      outerColor: { value: new THREE.Color(0xffffff) },
      innerOpacity: { value: 0.3 },
      outerOpacity: { value: 0.0 },
      falloffPower: { value: 1.2 },
      shadowRadius: { value: 7 },
    },
    vertexShader: `
      varying vec2 vUv;
      void main() {
        vUv = uv;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragmentShader: `
      uniform vec3 innerColor;
      uniform vec3 outerColor;
      uniform float innerOpacity;
      uniform float outerOpacity;
      uniform float falloffPower;
      uniform float shadowRadius;

      varying vec2 vUv;

      void main() {
        vec2 center = vec2(0.5, 0.5);
        float dist = distance(vUv, center) * 2.0;

        float shadowFalloff = 1.0 - smoothstep(0.0, shadowRadius, dist);
        float spotlightFalloff = 1.0 - smoothstep(0.0, 1.0, pow(dist, falloffPower));

        vec3 color = mix(outerColor, innerColor, shadowFalloff);
        float opacity = mix(outerOpacity, innerOpacity * shadowFalloff, spotlightFalloff);

        gl_FragColor = vec4(color, opacity);
      }
    `,
    transparent: true,
    depthWrite: false,
    depthTest: false,
  });
}

function installFlash(material: THREE.MeshStandardMaterial, intensity: number) {
  const flash = material as FlashMaterial;
  if (flash.userData.flashInstalled) return;

  const onBeforeCompile = material.onBeforeCompile.bind(material);
  const cacheKey = material.customProgramCacheKey.bind(material);

  material.onBeforeCompile = (shader, renderer) => {
    onBeforeCompile(shader, renderer);
    shader.uniforms.uFlashIntensity = { value: intensity };
    shader.uniforms.uFlashColor = { value: FLASH_COLOR };
    shader.fragmentShader = shader.fragmentShader
      .replace(
        '#include <common>',
        '#include <common>\nuniform float uFlashIntensity;\nuniform vec3 uFlashColor;',
      )
      .replace(
        '#include <dithering_fragment>',
        'gl_FragColor.rgb = mix(gl_FragColor.rgb, uFlashColor, uFlashIntensity * gl_FragColor.a);\n#include <dithering_fragment>',
      );
    flash.userData.flashShader = shader;
  };

  material.customProgramCacheKey = () => `${cacheKey()}|${FLASH_CACHE_KEY}`;
  flash.userData.flashInstalled = true;
  material.needsUpdate = true;
}

/** Push the current flash intensity into every skin material of the model. */
export function syncDamageFlash(
  model: THREE.Object3D | null,
  intensity: number,
): void {
  if (!model) return;

  model.traverse((child) => {
    const mesh = child as THREE.Mesh;
    if (!mesh.isMesh || !mesh.material) return;
    const materials = Array.isArray(mesh.material)
      ? mesh.material
      : [mesh.material];
    for (const material of materials) {
      if (!(material instanceof THREE.MeshStandardMaterial)) continue;
      if (material.name === 'cape') continue;
      installFlash(material, intensity);
      const shader = (material as FlashMaterial).userData.flashShader;
      if (shader) shader.uniforms.uFlashIntensity.value = intensity;
    }
  });
}

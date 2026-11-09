/**
 * Wex Launcher — Lightweight 3D Minecraft Character WebGL Renderer
 * 
 * Complies with:
 * - SPEC 1.2: Interactive 3D character model in Wardrobe
 * - Standard Minecraft 64x64 skin UV layout (Classic 4px & Slim 3px)
 * - Second layer (outer layer: hat, jacket, sleeves, pants)
 * - Official and custom capes
 * - Pointer / touch drag rotation with smooth damping inertia
 * - Respects prefers-reduced-motion (no auto-idle spin when reduced motion is preferred)
 * - Fallback to 2D canvas when WebGL is unavailable
 * - Pure WebGL implementation (0 npm dependencies, <15 KB)
 */

import { onIdleStateChange, isIdleSuspended } from './idleManager';

export type ModelType = 'default' | 'slim';

export interface UVRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface BoxUVFaces {
  top: UVRect;
  bottom: UVRect;
  right: UVRect;
  front: UVRect;
  left: UVRect;
  back: UVRect;
}

export interface PartDefinition {
  name: string;
  isOuter: boolean;
  size: [number, number, number]; // [width, height, depth]
  origin: [number, number, number]; // [centerX, centerY, centerZ]
  uv: BoxUVFaces;
}

/**
 * Parses standard Minecraft 64x64 skin UV coordinates for a cuboid.
 * Texture unfolding standard:
 * u, v: top-left of the box section
 * w, h, d: width, height, depth of the box in pixels
 */
export function makeBoxUV(u: number, v: number, w: number, h: number, d: number): BoxUVFaces {
  return {
    top: { x: u + d, y: v, w, h: d },
    bottom: { x: u + d + w, y: v, w, h: d },
    right: { x: u, y: v + d, w: d, h },
    front: { x: u + d, y: v + d, w, h },
    left: { x: u + d + w, y: v + d, w: d, h },
    back: { x: u + d + w + d, y: v + d, w, h },
  };
}

/**
 * Returns all character part definitions for Classic (4px) or Slim (3px) model.
 */
export function getCharacterPartDefinitions(model: ModelType): PartDefinition[] {
  const isSlim = model === 'slim';
  const armW = isSlim ? 3 : 4;
  const rightArmU = 40;
  const leftArmU = 32;

  // Outer layer expansion delta in 3D units
  const expand = 0.5;

  return [
    // 1. Head (8x8x8)
    {
      name: 'head',
      isOuter: false,
      size: [8, 8, 8],
      origin: [0, 16, 0],
      uv: makeBoxUV(0, 0, 8, 8, 8),
    },
    // Hat (Head outer layer)
    {
      name: 'hat',
      isOuter: true,
      size: [8 + expand, 8 + expand, 8 + expand],
      origin: [0, 16, 0],
      uv: makeBoxUV(32, 0, 8, 8, 8),
    },

    // 2. Torso (8x12x4)
    {
      name: 'torso',
      isOuter: false,
      size: [8, 12, 4],
      origin: [0, 6, 0],
      uv: makeBoxUV(16, 16, 8, 12, 4),
    },
    // Jacket (Torso outer layer)
    {
      name: 'jacket',
      isOuter: true,
      size: [8 + expand, 12 + expand, 4 + expand],
      origin: [0, 6, 0],
      uv: makeBoxUV(16, 32, 8, 12, 4),
    },

    // 3. Right Arm (armW x 12 x 4)
    // In Minecraft coordinates, right arm is on character's right side (-X)
    {
      name: 'rightArm',
      isOuter: false,
      size: [armW, 12, 4],
      origin: [isSlim ? -5.5 : -6, 6, 0],
      uv: makeBoxUV(rightArmU, 16, armW, 12, 4),
    },
    // Right Sleeve (outer layer)
    {
      name: 'rightSleeve',
      isOuter: true,
      size: [armW + expand, 12 + expand, 4 + expand],
      origin: [isSlim ? -5.5 : -6, 6, 0],
      uv: makeBoxUV(rightArmU, 32, armW, 12, 4),
    },

    // 4. Left Arm (armW x 12 x 4)
    {
      name: 'leftArm',
      isOuter: false,
      size: [armW, 12, 4],
      origin: [isSlim ? 5.5 : 6, 6, 0],
      uv: makeBoxUV(leftArmU, 48, armW, 12, 4),
    },
    // Left Sleeve (outer layer)
    {
      name: 'leftSleeve',
      isOuter: true,
      size: [armW + expand, 12 + expand, 4 + expand],
      origin: [isSlim ? 5.5 : 6, 6, 0],
      uv: makeBoxUV(48, 48, armW, 12, 4),
    },

    // 5. Right Leg (4x12x4)
    {
      name: 'rightLeg',
      isOuter: false,
      size: [4, 12, 4],
      origin: [-2, -6, 0],
      uv: makeBoxUV(0, 16, 4, 12, 4),
    },
    // Right Pants (outer layer)
    {
      name: 'rightPants',
      isOuter: true,
      size: [4 + expand, 12 + expand, 4 + expand],
      origin: [-2, -6, 0],
      uv: makeBoxUV(0, 32, 4, 12, 4),
    },

    // 6. Left Leg (4x12x4)
    {
      name: 'leftLeg',
      isOuter: false,
      size: [4, 12, 4],
      origin: [2, -6, 0],
      uv: makeBoxUV(16, 48, 4, 12, 4),
    },
    // Left Pants (outer layer)
    {
      name: 'leftPants',
      isOuter: true,
      size: [4 + expand, 12 + expand, 4 + expand],
      origin: [2, -6, 0],
      uv: makeBoxUV(0, 48, 4, 12, 4),
    },
  ];
}

/**
 * Returns Cape box definition (10x16x1)
 */
export function getCapePartDefinition(): PartDefinition {
  return {
    name: 'cape',
    isOuter: false,
    size: [10, 16, 1],
    origin: [0, 4, -2.5],
    uv: {
      top: { x: 1, y: 0, w: 10, h: 1 },
      bottom: { x: 11, y: 0, w: 10, h: 1 },
      right: { x: 0, y: 1, w: 1, h: 16 },
      front: { x: 12, y: 1, w: 10, h: 16 }, // Inside facing torso
      left: { x: 11, y: 1, w: 1, h: 16 },
      back: { x: 1, y: 1, w: 10, h: 16 }, // Exterior facing viewer from rear
    },
  };
}

/**
 * Checks if WebGL is available and functional on this canvas/system.
 */
export function isWebGLSupported(canvas?: HTMLCanvasElement | null): boolean {
  if (typeof window !== 'undefined' && typeof (window as any).WebGLRenderingContext === 'undefined') {
    if (!canvas || typeof (canvas as any).getContext !== 'function') return false;
    try {
      const gl = (canvas as any).getContext('webgl');
      return gl !== null && typeof (gl as any)?.createShader === 'function';
    } catch {
      return false;
    }
  }

  if (!canvas) {
    if (typeof document === 'undefined') return false;
    canvas = document.createElement('canvas');
  }
  try {
    const gl =
      canvas.getContext('webgl2') ||
      canvas.getContext('webgl') ||
      canvas.getContext('experimental-webgl');
    return gl !== null && typeof (gl as any)?.createShader === 'function';
  } catch {
    return false;
  }
}

/**
 * Pure 4x4 matrix utilities without external libraries.
 */
export class Mat4 {
  static identity(): Float32Array {
    const m = new Float32Array(16);
    m[0] = 1;
    m[5] = 1;
    m[10] = 1;
    m[15] = 1;
    return m;
  }

  static perspective(fovRad: number, aspect: number, near: number, far: number): Float32Array {
    const m = new Float32Array(16);
    const f = 1.0 / Math.tan(fovRad / 2);
    const nf = 1 / (near - far);

    m[0] = f / aspect;
    m[5] = f;
    m[10] = (far + near) * nf;
    m[11] = -1;
    m[14] = 2 * far * near * nf;
    return m;
  }

  static multiply(a: Float32Array, b: Float32Array): Float32Array {
    const out = new Float32Array(16);
    for (let i = 0; i < 4; i++) {
      const a0 = a[i];
      const a1 = a[i + 4];
      const a2 = a[i + 8];
      const a3 = a[i + 12];

      out[i] = a0 * b[0] + a1 * b[1] + a2 * b[2] + a3 * b[3];
      out[i + 4] = a0 * b[4] + a1 * b[5] + a2 * b[6] + a3 * b[7];
      out[i + 8] = a0 * b[8] + a1 * b[9] + a2 * b[10] + a3 * b[11];
      out[i + 12] = a0 * b[12] + a1 * b[13] + a2 * b[14] + a3 * b[15];
    }
    return out;
  }

  static translate(m: Float32Array, v: [number, number, number]): Float32Array {
    const out = new Float32Array(m);
    out[12] = m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12];
    out[13] = m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13];
    out[14] = m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14];
    out[15] = m[3] * v[0] + m[7] * v[1] + m[11] * v[2] + m[15];
    return out;
  }

  static rotateX(m: Float32Array, rad: number): Float32Array {
    const s = Math.sin(rad);
    const c = Math.cos(rad);
    const out = new Float32Array(m);

    const a10 = m[4], a11 = m[5], a12 = m[6], a13 = m[7];
    const a20 = m[8], a21 = m[9], a22 = m[10], a23 = m[11];

    out[4] = a10 * c + a20 * s;
    out[5] = a11 * c + a21 * s;
    out[6] = a12 * c + a22 * s;
    out[7] = a13 * c + a23 * s;

    out[8] = a20 * c - a10 * s;
    out[9] = a21 * c - a11 * s;
    out[10] = a22 * c - a12 * s;
    out[11] = a23 * c - a13 * s;

    return out;
  }

  static rotateY(m: Float32Array, rad: number): Float32Array {
    const s = Math.sin(rad);
    const c = Math.cos(rad);
    const out = new Float32Array(m);

    const a00 = m[0], a01 = m[1], a02 = m[2], a03 = m[3];
    const a20 = m[8], a21 = m[9], a22 = m[10], a23 = m[11];

    out[0] = a00 * c - a20 * s;
    out[1] = a01 * c - a21 * s;
    out[2] = a02 * c - a22 * s;
    out[3] = a03 * c - a23 * s;

    out[8] = a00 * s + a20 * c;
    out[9] = a01 * s + a21 * c;
    out[10] = a02 * s + a22 * c;
    out[11] = a03 * s + a23 * c;

    return out;
  }

  static rotateZ(m: Float32Array, rad: number): Float32Array {
    const s = Math.sin(rad);
    const c = Math.cos(rad);
    const out = new Float32Array(m);

    const a00 = m[0], a01 = m[1], a02 = m[2], a03 = m[3];
    const a10 = m[4], a11 = m[5], a12 = m[6], a13 = m[7];

    out[0] = a00 * c + a10 * s;
    out[1] = a01 * c + a10 * s;
    out[2] = a02 * c + a10 * s;
    out[3] = a03 * c + a10 * s;

    out[4] = a10 * c - a00 * s;
    out[5] = a11 * c - a01 * s;
    out[6] = a12 * c - a02 * s;
    out[7] = a13 * c - a03 * s;

    return out;
  }

  static rotateAroundPivot(
    m: Float32Array,
    pivot: [number, number, number],
    rx: number,
    ry = 0,
    rz = 0,
  ): Float32Array {
    if (rx === 0 && ry === 0 && rz === 0) return m;
    let out = Mat4.translate(m, pivot);
    if (rx !== 0) out = Mat4.rotateX(out, rx);
    if (ry !== 0) out = Mat4.rotateY(out, ry);
    if (rz !== 0) out = Mat4.rotateZ(out, rz);
    out = Mat4.translate(out, [-pivot[0], -pivot[1], -pivot[2]]);
    return out;
  }
}

/**
 * Generates vertex buffer data [posX, posY, posZ, normX, normY, normZ, u, v]
 * for a PartDefinition using given texture dimensions (e.g. 64x64 or 64x32 for cape).
 */
export function buildBoxVertices(part: PartDefinition, texW = 64, texH = 64): Float32Array {
  const [w, h, d] = part.size;
  const [ox, oy, oz] = part.origin;

  const minX = ox - w / 2;
  const maxX = ox + w / 2;
  const minY = oy - h / 2;
  const maxY = oy + h / 2;
  const minZ = oz - d / 2;
  const maxZ = oz + d / 2;

  const uv = part.uv;

  function toUV(rect: UVRect): [number, number, number, number] {
    const u0 = rect.x / texW;
    const v0 = rect.y / texH;
    const u1 = (rect.x + rect.w) / texW;
    const v1 = (rect.y + rect.h) / texH;
    return [u0, v0, u1, v1];
  }

  const [tU0, tV0, tU1, tV1] = toUV(uv.top);
  const [bU0, bV0, bU1, bV1] = toUV(uv.bottom);
  const [rU0, rV0, rU1, rV1] = toUV(uv.right);
  const [fU0, fV0, fU1, fV1] = toUV(uv.front);
  const [lU0, lV0, lU1, lV1] = toUV(uv.left);
  const [kU0, kV0, kU1, kV1] = toUV(uv.back);

  // 6 faces * 2 triangles * 3 vertices * 8 floats = 288 floats per box
  const verts: number[] = [
    // Front Face (Z = maxZ, normal 0, 0, 1)
    minX, minY, maxZ, 0, 0, 1, fU0, fV1,
    maxX, minY, maxZ, 0, 0, 1, fU1, fV1,
    maxX, maxY, maxZ, 0, 0, 1, fU1, fV0,
    minX, minY, maxZ, 0, 0, 1, fU0, fV1,
    maxX, maxY, maxZ, 0, 0, 1, fU1, fV0,
    minX, maxY, maxZ, 0, 0, 1, fU0, fV0,

    // Back Face (Z = minZ, normal 0, 0, -1)
    maxX, minY, minZ, 0, 0, -1, kU0, kV1,
    minX, minY, minZ, 0, 0, -1, kU1, kV1,
    minX, maxY, minZ, 0, 0, -1, kU1, kV0,
    maxX, minY, minZ, 0, 0, -1, kU0, kV1,
    minX, maxY, minZ, 0, 0, -1, kU1, kV0,
    maxX, maxY, minZ, 0, 0, -1, kU0, kV0,

    // Top Face (Y = maxY, normal 0, 1, 0)
    minX, maxY, maxZ, 0, 1, 0, tU0, tV1,
    maxX, maxY, maxZ, 0, 1, 0, tU1, tV1,
    maxX, maxY, minZ, 0, 1, 0, tU1, tV0,
    minX, maxY, maxZ, 0, 1, 0, tU0, tV1,
    maxX, maxY, minZ, 0, 1, 0, tU1, tV0,
    minX, maxY, minZ, 0, 1, 0, tU0, tV0,

    // Bottom Face (Y = minY, normal 0, -1, 0)
    minX, minY, minZ, 0, -1, 0, bU0, bV0,
    maxX, minY, minZ, 0, -1, 0, bU1, bV0,
    maxX, minY, maxZ, 0, -1, 0, bU1, bV1,
    minX, minY, minZ, 0, -1, 0, bU0, bV0,
    maxX, minY, maxZ, 0, -1, 0, bU1, bV1,
    minX, minY, maxZ, 0, -1, 0, bU0, bV1,

    // Right Face (X = maxX, normal 1, 0, 0)
    maxX, minY, maxZ, 1, 0, 0, rU0, rV1,
    maxX, minY, minZ, 1, 0, 0, rU1, rV1,
    maxX, maxY, minZ, 1, 0, 0, rU1, rV0,
    maxX, minY, maxZ, 1, 0, 0, rU0, rV1,
    maxX, maxY, minZ, 1, 0, 0, rU1, rV0,
    maxX, maxY, maxZ, 1, 0, 0, rU0, rV0,

    // Left Face (X = minX, normal -1, 0, 0)
    minX, minY, minZ, -1, 0, 0, lU0, lV1,
    minX, minY, maxZ, -1, 0, 0, lU1, lV1,
    minX, maxY, maxZ, -1, 0, 0, lU1, lV0,
    minX, minY, minZ, -1, 0, 0, lU0, lV1,
    minX, maxY, maxZ, -1, 0, 0, lU1, lV0,
    minX, maxY, minZ, -1, 0, 0, lU0, lV0,
  ];

  return new Float32Array(verts);
}

/**
 * Builds complete character mesh buffer for given model.
 */
export function buildCharacterModelBuffer(model: ModelType): {
  buffer: Float32Array;
  vertexCount: number;
} {
  const parts = getCharacterPartDefinitions(model);
  const partBuffers = parts.map((p) => buildBoxVertices(p, 64, 64));

  let totalFloats = 0;
  for (const b of partBuffers) totalFloats += b.length;

  const combined = new Float32Array(totalFloats);
  let offset = 0;
  for (const b of partBuffers) {
    combined.set(b, offset);
    offset += b.length;
  }

  return {
    buffer: combined,
    vertexCount: totalFloats / 8, // 8 floats per vertex
  };
}

/**
 * Builds cape mesh buffer.
 */
export function buildCapeModelBuffer(): {
  buffer: Float32Array;
  vertexCount: number;
} {
  const cape = getCapePartDefinition();
  const buffer = buildBoxVertices(cape, 64, 32);
  return {
    buffer,
    vertexCount: buffer.length / 8,
  };
}

export type AnimationMode = 'idle' | 'walking';

export interface LimbGeometry {
  name: 'head' | 'torso' | 'rightArm' | 'leftArm' | 'rightLeg' | 'leftLeg';
  pivot: [number, number, number];
  buffer: Float32Array;
  vertexCount: number;
}

/**
 * Builds separate limb meshes (pairing inner and outer 3D layers)
 * so limbs can rotate around their respective anatomical joints for walking/idle animations.
 */
export function buildCharacterLimbGeometries(model: ModelType): LimbGeometry[] {
  const parts = getCharacterPartDefinitions(model);
  const isSlim = model === 'slim';
  const armX = isSlim ? 5.5 : 6.0;

  function combine(partList: PartDefinition[]): { buffer: Float32Array; vertexCount: number } {
    const buffers = partList.map((p) => buildBoxVertices(p, 64, 64));
    let totalLen = 0;
    for (const b of buffers) totalLen += b.length;
    const res = new Float32Array(totalLen);
    let off = 0;
    for (const b of buffers) {
      res.set(b, off);
      off += b.length;
    }
    return { buffer: res, vertexCount: totalLen / 8 };
  }

  const head = combine(parts.filter((p) => p.name === 'head' || p.name === 'hat'));
  const torso = combine(parts.filter((p) => p.name === 'torso' || p.name === 'jacket'));
  const rightArm = combine(parts.filter((p) => p.name === 'rightArm' || p.name === 'rightSleeve'));
  const leftArm = combine(parts.filter((p) => p.name === 'leftArm' || p.name === 'leftSleeve'));
  const rightLeg = combine(parts.filter((p) => p.name === 'rightLeg' || p.name === 'rightPants'));
  const leftLeg = combine(parts.filter((p) => p.name === 'leftLeg' || p.name === 'leftPants'));

  return [
    { name: 'head', pivot: [0, 12, 0], buffer: head.buffer, vertexCount: head.vertexCount },
    { name: 'torso', pivot: [0, 6, 0], buffer: torso.buffer, vertexCount: torso.vertexCount },
    { name: 'rightArm', pivot: [-armX, 12, 0], buffer: rightArm.buffer, vertexCount: rightArm.vertexCount },
    { name: 'leftArm', pivot: [armX, 12, 0], buffer: leftArm.buffer, vertexCount: leftArm.vertexCount },
    { name: 'rightLeg', pivot: [-2, 0, 0], buffer: rightLeg.buffer, vertexCount: rightLeg.vertexCount },
    { name: 'leftLeg', pivot: [2, 0, 0], buffer: leftLeg.buffer, vertexCount: leftLeg.vertexCount },
  ];
}

const VERTEX_SHADER_SRC = `
attribute vec3 a_position;
attribute vec3 a_normal;
attribute vec2 a_texcoord;

uniform mat4 u_matrix;
uniform mat4 u_world;

varying vec2 v_texcoord;
varying vec3 v_normal;

void main() {
  gl_Position = u_matrix * vec4(a_position, 1.0);
  v_texcoord = a_texcoord;
  v_normal = normalize(mat3(u_world) * a_normal);
}
`;

const FRAGMENT_SHADER_SRC = `
precision mediump float;

varying vec2 v_texcoord;
varying vec3 v_normal;

uniform sampler2D u_texture;
uniform sampler2D u_textureNext;
uniform float u_textureMix;
uniform vec3 u_lightDir;

void main() {
  vec4 colorCurr = texture2D(u_texture, v_texcoord);
  vec4 colorNext = texture2D(u_textureNext, v_texcoord);
  vec4 color = mix(colorCurr, colorNext, u_textureMix);
  if (color.a < 0.1) {
    discard;
  }

  // Apple HIG-compliant subtle lighting (warm neutral key light)
  float light = dot(v_normal, normalize(u_lightDir));
  float diffuse = max(light, 0.0) * 0.35;
  float ambient = 0.75;
  vec3 lit = color.rgb * (ambient + diffuse);

  gl_FragColor = vec4(lit, color.a);
}
`;

export interface RendererOptions {
  canvas: HTMLCanvasElement;
  model?: ModelType;
  skinImage?: HTMLImageElement | HTMLCanvasElement | null;
  capeImage?: HTMLImageElement | HTMLCanvasElement | null;
  hasCape?: boolean;
  animation?: AnimationMode;
  onRotationChange?: (yaw: number, pitch: number) => void;
}

export class Skin3DRenderer {
  private gl: WebGLRenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private charVaoBuffer: WebGLBuffer | null = null;
  private capeVaoBuffer: WebGLBuffer | null = null;
  private limbBuffers: Map<string, { buffer: WebGLBuffer; vertexCount: number; pivot: [number, number, number] }> = new Map();
  private skinTexture: WebGLTexture | null = null;
  private skinTextureNext: WebGLTexture | null = null;
  private capeTexture: WebGLTexture | null = null;
  private hasActiveSkin = false;
  private isTransitioning = false;
  private currentTextureMix = 0.0;
  private transitionStartTime = 0;

  private charVertexCount = 0;
  private capeVertexCount = 0;

  private model: ModelType = 'default';
  private hasCape = false;
  private _animation: AnimationMode = 'idle';
  private _frameCount = 0;

  // Interaction & inertia state
  public yaw = 0.45; // Start angled towards 3/4 view
  public pitch = 0.1;
  public velocityX = 0;
  public velocityY = 0;
  public isDragging = false;
  private lastPointerX = 0;
  private lastPointerY = 0;

  private prefersReducedMotion = false;
  private animationId: number | null = null;
  private destroyed = false;
  private _isPaused = false;
  private cleanupIdleListener: () => void = () => {};

  constructor(private options: RendererOptions) {
    this.model = options.model || 'default';
    this.hasCape = !!options.hasCape;
    this._animation = options.animation || 'idle';

    if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
      const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
      this.prefersReducedMotion = mq.matches;
      mq.addEventListener?.('change', (e) => {
        this.prefersReducedMotion = e.matches;
      });
    }

    this._isPaused = isIdleSuspended();
    this.cleanupIdleListener = onIdleStateChange((suspended) => {
      if (suspended) {
        this.pause();
      } else {
        this.resume();
      }
    });

    this.initGL();
    this.initListeners();
    this.startLoop();
  }

  public get frameCount(): number {
    return this._frameCount;
  }

  public getFrameCount(): number {
    return this._frameCount;
  }

  public get isPaused(): boolean {
    return this._isPaused;
  }

  public isRenderPaused(): boolean {
    return this._isPaused;
  }

  public get animation(): AnimationMode {
    return this._animation;
  }

  public getAnimation(): AnimationMode {
    return this._animation;
  }

  public setAnimation(anim: AnimationMode): void {
    this._animation = anim;
  }

  private initGL(): boolean {
    const canvas = this.options.canvas;
    const gl =
      canvas.getContext('webgl', { alpha: true, antialias: true }) ||
      (canvas.getContext('experimental-webgl', {
        alpha: true,
        antialias: true,
      }) as WebGLRenderingContext | null);

    if (!gl) return false;
    this.gl = gl;

    // Create Shaders & Program
    const vs = this.compileShader(gl.VERTEX_SHADER, VERTEX_SHADER_SRC);
    const fs = this.compileShader(gl.FRAGMENT_SHADER, FRAGMENT_SHADER_SRC);
    if (!vs || !fs) return false;

    const prog = gl.createProgram();
    if (!prog) return false;
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);

    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      gl.deleteProgram(prog);
      return false;
    }
    this.program = prog;

    // GL State
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.LEQUAL);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

    // Build geometry buffers
    this.updateModelGeometry(this.model);

    // Textures
    this.skinTexture = gl.createTexture();
    this.skinTextureNext = gl.createTexture();
    this.capeTexture = gl.createTexture();

    if (this.options.skinImage) {
      this.setSkinImage(this.options.skinImage);
    }
    if (this.options.capeImage) {
      this.setCapeImage(this.options.capeImage);
    }

    return true;
  }

  public updateModelGeometry(model: ModelType) {
    if (!this.gl) return;
    const gl = this.gl;
    this.model = model;

    // 1. Character single buffer for backward compatibility
    const charData = buildCharacterModelBuffer(model);
    this.charVertexCount = charData.vertexCount;

    if (!this.charVaoBuffer) {
      this.charVaoBuffer = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.charVaoBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, charData.buffer, gl.STATIC_DRAW);

    // 2. Animated Limb buffers with joint pivots
    const limbs = buildCharacterLimbGeometries(model);
    for (const limb of limbs) {
      let existing = this.limbBuffers.get(limb.name);
      if (!existing) {
        const buf = gl.createBuffer();
        if (buf) {
          existing = { buffer: buf, vertexCount: limb.vertexCount, pivot: limb.pivot };
          this.limbBuffers.set(limb.name, existing);
        }
      }
      if (existing) {
        existing.vertexCount = limb.vertexCount;
        existing.pivot = limb.pivot;
        gl.bindBuffer(gl.ARRAY_BUFFER, existing.buffer);
        gl.bufferData(gl.ARRAY_BUFFER, limb.buffer, gl.STATIC_DRAW);
      }
    }

    // 3. Cape buffer
    const capeData = buildCapeModelBuffer();
    this.capeVertexCount = capeData.vertexCount;

    if (!this.capeVaoBuffer) {
      this.capeVaoBuffer = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.capeVaoBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, capeData.buffer, gl.STATIC_DRAW);
  }

  private uploadTexture(tex: WebGLTexture, img: HTMLImageElement | HTMLCanvasElement) {
    if (!this.gl) return;
    const gl = this.gl;
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);

    // Crisp pixelated Minecraft texture filtering
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  public setSkinImage(img: HTMLImageElement | HTMLCanvasElement) {
    if (!this.gl) return;
    if (!this.skinTexture) this.skinTexture = this.gl.createTexture();
    if (!this.skinTextureNext) this.skinTextureNext = this.gl.createTexture();
    if (!this.skinTexture || !this.skinTextureNext) return;

    if (!this.hasActiveSkin) {
      this.uploadTexture(this.skinTexture, img);
      this.uploadTexture(this.skinTextureNext, img);
      this.hasActiveSkin = true;
      this.isTransitioning = false;
      this.currentTextureMix = 0.0;
    } else {
      // Upload to next texture and begin smooth 220ms crossfade
      this.uploadTexture(this.skinTextureNext, img);
      this.transitionStartTime = performance.now();
      this.isTransitioning = true;
    }
  }

  public setCapeImage(img: HTMLImageElement | HTMLCanvasElement | null) {
    if (!this.gl || !this.capeTexture) return;
    const gl = this.gl;
    if (!img) {
      this.hasCape = false;
      return;
    }

    this.uploadTexture(this.capeTexture, img);
    this.hasCape = true;
  }

  public setHasCape(active: boolean) {
    this.hasCape = active;
  }

  public resetOrientation() {
    this.yaw = 0.45;
    this.pitch = 0.1;
    this.velocityX = 0;
    this.velocityY = 0;
  }

  private compileShader(type: number, src: string): WebGLShader | null {
    if (!this.gl) return null;
    const shader = this.gl.createShader(type);
    if (!shader) return null;
    this.gl.shaderSource(shader, src);
    this.gl.compileShader(shader);
    if (!this.gl.getShaderParameter(shader, this.gl.COMPILE_STATUS)) {
      this.gl.deleteShader(shader);
      return null;
    }
    return shader;
  }

  private initListeners() {
    const canvas = this.options.canvas;

    const onPointerDown = (e: PointerEvent) => {
      this.isDragging = true;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;
      this.velocityX = 0;
      this.velocityY = 0;
      canvas.setPointerCapture?.(e.pointerId);
    };

    const onPointerMove = (e: PointerEvent) => {
      if (!this.isDragging) return;
      const dx = e.clientX - this.lastPointerX;
      const dy = e.clientY - this.lastPointerY;
      this.lastPointerX = e.clientX;
      this.lastPointerY = e.clientY;

      // Sensitivity factor
      const sensitivity = 0.012;
      this.yaw += dx * sensitivity;
      this.pitch += dy * sensitivity;

      // Clamp vertical pitch to prevent inverting view
      this.pitch = Math.max(-0.8, Math.min(0.8, this.pitch));

      // Record velocity for release inertia
      this.velocityX = dx * sensitivity;
      this.velocityY = dy * sensitivity;

      this.options.onRotationChange?.(this.yaw, this.pitch);
    };

    const onPointerUp = (e: PointerEvent) => {
      this.isDragging = false;
      canvas.releasePointerCapture?.(e.pointerId);
    };

    canvas.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);
    canvas.addEventListener('pointercancel', onPointerUp);

    this.cleanupListeners = () => {
      canvas.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
      canvas.removeEventListener('pointercancel', onPointerUp);
    };
  }

  private cleanupListeners: () => void = () => {};

  public pause(): void {
    this._isPaused = true;
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  public resume(): void {
    if (this.destroyed || !this._isPaused) return;
    this._isPaused = false;
    if (this.animationId === null) {
      this.startLoop();
    }
  }

  private startLoop() {
    if (this.destroyed || this._isPaused) return;
    const renderFrame = () => {
      if (this.destroyed || this._isPaused) {
        this.animationId = null;
        return;
      }

      this.stepPhysics();
      this.render();

      this.animationId = requestAnimationFrame(renderFrame);
    };
    this.animationId = requestAnimationFrame(renderFrame);
  }

  private stepPhysics() {
    if (!this.isDragging) {
      // Damping inertia
      this.yaw += this.velocityX;
      this.pitch += this.velocityY;

      this.velocityX *= 0.92;
      this.velocityY *= 0.92;

      this.pitch = Math.max(-0.8, Math.min(0.8, this.pitch));

      // Idle auto-rotation ONLY when not reduced motion and not interacting
      if (!this.prefersReducedMotion && Math.abs(this.velocityX) < 0.001) {
        this.yaw += 0.004; // Gentle turntable rotation
      }
    }
  }

  public render() {
    if (!this.gl || !this.program) return;
    this._frameCount++;
    const gl = this.gl;
    const canvas = this.options.canvas;

    // Handle high DPI display
    const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    const displayWidth = Math.round(canvas.clientWidth * dpr);
    const displayHeight = Math.round(canvas.clientHeight * dpr);

    if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
      canvas.width = displayWidth;
      canvas.height = displayHeight;
      gl.viewport(0, 0, displayWidth, displayHeight);
    }

    gl.clearColor(0.0, 0.0, 0.0, 0.0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    gl.useProgram(this.program);

    // Uniform locations
    const uMatrix = gl.getUniformLocation(this.program, 'u_matrix');
    const uWorld = gl.getUniformLocation(this.program, 'u_world');
    const uLightDir = gl.getUniformLocation(this.program, 'u_lightDir');
    const uTexture = gl.getUniformLocation(this.program, 'u_texture');
    const uTextureNext = gl.getUniformLocation(this.program, 'u_textureNext');
    const uTextureMix = gl.getUniformLocation(this.program, 'u_textureMix');

    // Directional light from front-top-right (classic isometric portrait light)
    gl.uniform3f(uLightDir, 0.5, 0.8, 1.0);

    // Camera & Transform
    const aspect = canvas.width / canvas.height;
    const proj = Mat4.perspective((34 * Math.PI) / 180, aspect, 1, 100);

    // Position camera back to fit 32 unit model nicely
    let view = Mat4.translate(Mat4.identity(), [0, -3.5, -54]);
    view = Mat4.rotateX(view, this.pitch);
    view = Mat4.rotateY(view, this.yaw);

    // Setup attributes
    const aPos = gl.getAttribLocation(this.program, 'a_position');
    const aNorm = gl.getAttribLocation(this.program, 'a_normal');
    const aTex = gl.getAttribLocation(this.program, 'a_texcoord');

    const stride = 8 * Float32Array.BYTES_PER_ELEMENT;

    // Handle smooth texture transition progress (crossfade without flicker)
    if (this.isTransitioning) {
      const elapsed = performance.now() - this.transitionStartTime;
      const duration = 220; // 220ms smooth crossfade
      const progress = Math.min(1.0, elapsed / duration);
      this.currentTextureMix = 1.0 - Math.pow(1.0 - progress, 3); // cubic ease-out
      if (progress >= 1.0) {
        this.isTransitioning = false;
        this.currentTextureMix = 0.0;
        // Swap textures so skinTexture holds new image
        const tmp = this.skinTexture;
        this.skinTexture = this.skinTextureNext;
        this.skinTextureNext = tmp;
      }
    } else {
      this.currentTextureMix = 0.0;
    }

    // Animation rotations
    const t = performance.now() / 1000;
    let headRot = { rx: 0, ry: 0, rz: 0 };
    let torsoRot = { rx: 0, ry: 0, rz: 0 };
    let rightArmRot = { rx: 0, ry: 0, rz: 0 };
    let leftArmRot = { rx: 0, ry: 0, rz: 0 };
    let rightLegRot = { rx: 0, ry: 0, rz: 0 };
    let leftLegRot = { rx: 0, ry: 0, rz: 0 };
    let capeRot = { rx: 0.16, ry: 0, rz: 0 };

    if (!this.prefersReducedMotion) {
      if (this._animation === 'walking') {
        const speed = 4.5;
        const legSwing = Math.sin(t * speed) * 0.55;
        rightLegRot.rx = legSwing;
        leftLegRot.rx = -legSwing;
        rightArmRot.rx = -legSwing;
        rightArmRot.rz = Math.cos(t * speed * 0.5) * 0.04 + 0.05;
        leftArmRot.rx = legSwing;
        leftArmRot.rz = -Math.cos(t * speed * 0.5) * 0.04 - 0.05;
        headRot.rx = Math.sin(t * speed * 2) * 0.04;
        torsoRot.ry = Math.sin(t * speed) * 0.03;
        capeRot.rx = 0.35 + Math.abs(Math.sin(t * speed)) * 0.22;
      } else {
        // Idle
        const speed = 1.6;
        const breath = Math.sin(t * speed);
        const look = Math.cos(t * speed * 0.6);
        headRot.rx = breath * 0.03;
        headRot.ry = look * 0.04;
        rightArmRot.rx = Math.cos(t * speed) * 0.04 + 0.06;
        rightArmRot.rz = breath * 0.03 + 0.05;
        leftArmRot.rx = -Math.cos(t * speed) * 0.04 + 0.06;
        leftArmRot.rz = -breath * 0.03 - 0.05;
        capeRot.rx = 0.14 + breath * 0.03;
      }
    }

    const limbRotMap: Record<string, { rx: number; ry: number; rz: number }> = {
      head: headRot,
      torso: torsoRot,
      rightArm: rightArmRot,
      leftArm: leftArmRot,
      rightLeg: rightLegRot,
      leftLeg: leftLegRot,
    };

    // 1. Render Character Limbs with joint pivots
    if (this.skinTexture && this.limbBuffers.size > 0) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.skinTexture);
      gl.uniform1i(uTexture, 0);

      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, this.skinTextureNext || this.skinTexture);
      gl.uniform1i(uTextureNext, 1);
      gl.uniform1f(uTextureMix, this.currentTextureMix);

      for (const [name, limb] of this.limbBuffers.entries()) {
        const rot = limbRotMap[name] || { rx: 0, ry: 0, rz: 0 };
        const limbView = Mat4.rotateAroundPivot(view, limb.pivot, rot.rx, rot.ry, rot.rz);
        const limbMvp = Mat4.multiply(proj, limbView);

        gl.uniformMatrix4fv(uMatrix, false, limbMvp);
        gl.uniformMatrix4fv(uWorld, false, limbView);

        gl.bindBuffer(gl.ARRAY_BUFFER, limb.buffer);
        gl.enableVertexAttribArray(aPos);
        gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, stride, 0);

        gl.enableVertexAttribArray(aNorm);
        gl.vertexAttribPointer(aNorm, 3, gl.FLOAT, false, stride, 3 * Float32Array.BYTES_PER_ELEMENT);

        gl.enableVertexAttribArray(aTex);
        gl.vertexAttribPointer(aTex, 2, gl.FLOAT, false, stride, 6 * Float32Array.BYTES_PER_ELEMENT);

        gl.drawArrays(gl.TRIANGLES, 0, limb.vertexCount);
      }
    } else if (this.charVaoBuffer && this.skinTexture) {
      // Fallback single-buffer render
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.skinTexture);
      gl.uniform1i(uTexture, 0);
      gl.uniform1f(uTextureMix, 0.0);

      const mvp = Mat4.multiply(proj, view);
      gl.uniformMatrix4fv(uMatrix, false, mvp);
      gl.uniformMatrix4fv(uWorld, false, view);

      gl.bindBuffer(gl.ARRAY_BUFFER, this.charVaoBuffer);
      gl.enableVertexAttribArray(aPos);
      gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, stride, 0);
      gl.enableVertexAttribArray(aNorm);
      gl.vertexAttribPointer(aNorm, 3, gl.FLOAT, false, stride, 3 * Float32Array.BYTES_PER_ELEMENT);
      gl.enableVertexAttribArray(aTex);
      gl.vertexAttribPointer(aTex, 2, gl.FLOAT, false, stride, 6 * Float32Array.BYTES_PER_ELEMENT);

      gl.drawArrays(gl.TRIANGLES, 0, this.charVertexCount);
    }

    // 2. Render Cape (if attached)
    if (this.hasCape && this.capeVaoBuffer && this.capeTexture) {
      const capePivot: [number, number, number] = [0, 12, -2];
      const capeView = Mat4.rotateAroundPivot(view, capePivot, capeRot.rx, capeRot.ry, capeRot.rz);
      const capeMvp = Mat4.multiply(proj, capeView);

      gl.uniformMatrix4fv(uMatrix, false, capeMvp);
      gl.uniformMatrix4fv(uWorld, false, capeView);

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.capeTexture);
      gl.uniform1i(uTexture, 0);
      gl.uniform1f(uTextureMix, 0.0);

      gl.bindBuffer(gl.ARRAY_BUFFER, this.capeVaoBuffer);
      gl.enableVertexAttribArray(aPos);
      gl.vertexAttribPointer(aPos, 3, gl.FLOAT, false, stride, 0);

      gl.enableVertexAttribArray(aNorm);
      gl.vertexAttribPointer(aNorm, 3, gl.FLOAT, false, stride, 3 * Float32Array.BYTES_PER_ELEMENT);

      gl.enableVertexAttribArray(aTex);
      gl.vertexAttribPointer(aTex, 2, gl.FLOAT, false, stride, 6 * Float32Array.BYTES_PER_ELEMENT);

      gl.drawArrays(gl.TRIANGLES, 0, this.capeVertexCount);
    }
  }

  public destroy() {
    this.destroyed = true;
    this.pause();
    this.cleanupIdleListener();
    this.cleanupListeners();

    if (this.gl) {
      for (const limb of this.limbBuffers.values()) {
        this.gl.deleteBuffer(limb.buffer);
      }
      this.limbBuffers.clear();
      if (this.charVaoBuffer) this.gl.deleteBuffer(this.charVaoBuffer);
      if (this.capeVaoBuffer) this.gl.deleteBuffer(this.capeVaoBuffer);
      if (this.skinTexture) this.gl.deleteTexture(this.skinTexture);
      if (this.skinTextureNext) this.gl.deleteTexture(this.skinTextureNext);
      if (this.capeTexture) this.gl.deleteTexture(this.capeTexture);
      if (this.program) this.gl.deleteProgram(this.program);
    }
  }
}

/**
 * Creates an in-memory 64x64 canvas with standard Minecraft default character texture (Steve or Alex).
 */
export function createDefaultSkinCanvas(model: ModelType): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  const ctx = canvas.getContext('2d');
  if (!ctx) return canvas;

  ctx.imageSmoothingEnabled = false;
  const isSlim = model === 'slim';

  const skinColor = isSlim ? '#E4B490' : '#A77D5A';
  const hairColor = isSlim ? '#C25D25' : '#4A321E'; // Alex orange vs Steve brown
  const shirtColor = isSlim ? '#5B8731' : '#00AAAA'; // Alex green vs Steve cyan
  const pantsColor = isSlim ? '#4F3E2F' : '#26348C'; // Alex brown vs Steve blue
  const shoesColor = isSlim ? '#3A3026' : '#5A5A5A';

  // 1. Head (0, 0 to 32, 16)
  // Base skin
  ctx.fillStyle = skinColor;
  ctx.fillRect(0, 0, 32, 16);
  // Hair on top and sides
  ctx.fillStyle = hairColor;
  ctx.fillRect(0, 0, 32, 8); // Top, bottom, sides
  ctx.fillRect(8, 8, 8, 2); // Front hair
  ctx.fillRect(24, 8, 8, 8); // Back hair
  // Eyes
  ctx.fillStyle = '#FFFFFF';
  ctx.fillRect(9, 11, 2, 1);
  ctx.fillRect(13, 11, 2, 1);
  ctx.fillStyle = isSlim ? '#26654A' : '#3C4DA2';
  ctx.fillRect(10, 11, 1, 1);
  ctx.fillRect(13, 11, 1, 1);
  // Nose / mouth
  ctx.fillStyle = isSlim ? '#BD8B6B' : '#6E452B';
  ctx.fillRect(11, 13, 2, 1);

  // 2. Torso (16, 16 to 40, 32)
  ctx.fillStyle = shirtColor;
  ctx.fillRect(16, 16, 24, 16);

  // 3. Right Arm (40, 16 to 56, 32)
  ctx.fillStyle = shirtColor;
  ctx.fillRect(40, 16, 16, 6);
  ctx.fillStyle = skinColor;
  ctx.fillRect(40, 22, 16, 10);

  // 4. Left Arm (32, 48 to 48, 64)
  ctx.fillStyle = shirtColor;
  ctx.fillRect(32, 48, 16, 6);
  ctx.fillStyle = skinColor;
  ctx.fillRect(32, 54, 16, 10);

  // 5. Right Leg (0, 16 to 16, 32)
  ctx.fillStyle = pantsColor;
  ctx.fillRect(0, 16, 16, 14);
  ctx.fillStyle = shoesColor;
  ctx.fillRect(0, 30, 16, 2);

  // 6. Left Leg (16, 48 to 32, 64)
  ctx.fillStyle = pantsColor;
  ctx.fillRect(16, 48, 16, 14);
  ctx.fillStyle = shoesColor;
  ctx.fillRect(16, 62, 16, 2);

  return canvas;
}

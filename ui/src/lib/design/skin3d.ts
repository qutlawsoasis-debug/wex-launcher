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
uniform vec3 u_lightDir;

void main() {
  vec4 color = texture2D(u_texture, v_texcoord);
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
  onRotationChange?: (yaw: number, pitch: number) => void;
}

export class Skin3DRenderer {
  private gl: WebGLRenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private charVaoBuffer: WebGLBuffer | null = null;
  private capeVaoBuffer: WebGLBuffer | null = null;
  private skinTexture: WebGLTexture | null = null;
  private capeTexture: WebGLTexture | null = null;

  private charVertexCount = 0;
  private capeVertexCount = 0;

  private model: ModelType = 'default';
  private hasCape = false;

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
  private isPaused = false;
  private cleanupIdleListener: () => void = () => {};

  constructor(private options: RendererOptions) {
    this.model = options.model || 'default';
    this.hasCape = !!options.hasCape;

    if (typeof window !== 'undefined') {
      const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
      this.prefersReducedMotion = mq.matches;
      mq.addEventListener?.('change', (e) => {
        this.prefersReducedMotion = e.matches;
      });
    }

    this.isPaused = isIdleSuspended();
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

    // Character buffer
    const charData = buildCharacterModelBuffer(model);
    this.charVertexCount = charData.vertexCount;

    if (!this.charVaoBuffer) {
      this.charVaoBuffer = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.charVaoBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, charData.buffer, gl.STATIC_DRAW);

    // Cape buffer
    const capeData = buildCapeModelBuffer();
    this.capeVertexCount = capeData.vertexCount;

    if (!this.capeVaoBuffer) {
      this.capeVaoBuffer = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.capeVaoBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, capeData.buffer, gl.STATIC_DRAW);
  }

  public setSkinImage(img: HTMLImageElement | HTMLCanvasElement) {
    if (!this.gl || !this.skinTexture) return;
    const gl = this.gl;

    gl.bindTexture(gl.TEXTURE_2D, this.skinTexture);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);

    // Crisp pixelated Minecraft texture filtering
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  public setCapeImage(img: HTMLImageElement | HTMLCanvasElement | null) {
    if (!this.gl || !this.capeTexture) return;
    const gl = this.gl;
    if (!img) {
      this.hasCape = false;
      return;
    }

    gl.bindTexture(gl.TEXTURE_2D, this.capeTexture);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);

    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

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
    this.isPaused = true;
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
  }

  public resume(): void {
    if (this.destroyed || !this.isPaused) return;
    this.isPaused = false;
    if (this.animationId === null) {
      this.startLoop();
    }
  }

  private startLoop() {
    if (this.destroyed || this.isPaused) return;
    const renderFrame = () => {
      if (this.destroyed || this.isPaused) {
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
        this.yaw += 0.005; // Gentle turntable rotation
      }
    }
  }

  public render() {
    if (!this.gl || !this.program) return;
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

    // Directional light from front-top-right (classic isometric portrait light)
    gl.uniform3f(uLightDir, 0.5, 0.8, 1.0);
    gl.uniform1i(uTexture, 0);

    // Camera & Transform
    const aspect = canvas.width / canvas.height;
    const proj = Mat4.perspective((34 * Math.PI) / 180, aspect, 1, 100);

    // Position camera back to fit 32 unit model nicely
    let view = Mat4.translate(Mat4.identity(), [0, -3.5, -54]);
    view = Mat4.rotateX(view, this.pitch);
    view = Mat4.rotateY(view, this.yaw);

    const mvp = Mat4.multiply(proj, view);

    gl.uniformMatrix4fv(uMatrix, false, mvp);
    gl.uniformMatrix4fv(uWorld, false, view);

    // Setup attributes
    const aPos = gl.getAttribLocation(this.program, 'a_position');
    const aNorm = gl.getAttribLocation(this.program, 'a_normal');
    const aTex = gl.getAttribLocation(this.program, 'a_texcoord');

    const stride = 8 * Float32Array.BYTES_PER_ELEMENT;

    // 1. Render Character Body
    if (this.charVaoBuffer && this.skinTexture) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.skinTexture);

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
      // Cape hangs at back of shoulders with slight backwards angle
      let capeView = Mat4.rotateX(view, 0.16); // ~9 degrees tilt back
      const capeMvp = Mat4.multiply(proj, capeView);
      gl.uniformMatrix4fv(uMatrix, false, capeMvp);
      gl.uniformMatrix4fv(uWorld, false, capeView);

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, this.capeTexture);

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
    this.cleanupIdleListener();
    if (this.animationId !== null) {
      cancelAnimationFrame(this.animationId);
      this.animationId = null;
    }
    this.cleanupListeners();

    if (this.gl) {
      if (this.charVaoBuffer) this.gl.deleteBuffer(this.charVaoBuffer);
      if (this.capeVaoBuffer) this.gl.deleteBuffer(this.capeVaoBuffer);
      if (this.skinTexture) this.gl.deleteTexture(this.skinTexture);
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

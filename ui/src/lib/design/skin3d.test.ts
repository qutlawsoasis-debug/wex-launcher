import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  makeBoxUV,
  getCharacterPartDefinitions,
  getCapePartDefinition,
  buildBoxVertices,
  buildCharacterModelBuffer,
  buildCapeModelBuffer,
  buildCharacterLimbGeometries,
  isWebGLSupported,
  createDefaultSkinCanvas,
  Skin3DRenderer,
  Mat4,
} from './skin3d';
import { setGameRunning } from './idleManager';

describe('Skin 3D UV Layout & Geometry Parser (SPEC 1.2)', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation((...args) => {
      const msg = String(args[0]?.message || args[0]);
      if (msg.includes('Not implemented: HTMLCanvasElement')) return;
    });
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });
  it('correctly calculates Minecraft box UV layout unfolding', () => {
    // Head box: w=8, h=8, d=8 at (0, 0)
    const headUV = makeBoxUV(0, 0, 8, 8, 8);

    expect(headUV.top).toEqual({ x: 8, y: 0, w: 8, h: 8 });
    expect(headUV.bottom).toEqual({ x: 16, y: 0, w: 8, h: 8 });
    expect(headUV.right).toEqual({ x: 0, y: 8, w: 8, h: 8 });
    expect(headUV.front).toEqual({ x: 8, y: 8, w: 8, h: 8 });
    expect(headUV.left).toEqual({ x: 16, y: 8, w: 8, h: 8 });
    expect(headUV.back).toEqual({ x: 24, y: 8, w: 8, h: 8 });

    // Torso box: w=8, h=12, d=4 at (16, 16)
    const torsoUV = makeBoxUV(16, 16, 8, 12, 4);
    expect(torsoUV.top).toEqual({ x: 20, y: 16, w: 8, h: 4 });
    expect(torsoUV.bottom).toEqual({ x: 28, y: 16, w: 8, h: 4 });
    expect(torsoUV.right).toEqual({ x: 16, y: 20, w: 4, h: 12 });
    expect(torsoUV.front).toEqual({ x: 20, y: 20, w: 8, h: 12 });
    expect(torsoUV.left).toEqual({ x: 28, y: 20, w: 4, h: 12 });
    expect(torsoUV.back).toEqual({ x: 32, y: 20, w: 8, h: 12 });
  });

  it('differentiates Classic (4px) vs Slim (3px) model arm dimensions and positions', () => {
    const classicParts = getCharacterPartDefinitions('default');
    const slimParts = getCharacterPartDefinitions('slim');

    expect(classicParts.length).toBe(12);
    expect(slimParts.length).toBe(12);

    // Classic Right Arm (4px width)
    const classicRA = classicParts.find((p) => p.name === 'rightArm')!;
    expect(classicRA.size).toEqual([4, 12, 4]);
    expect(classicRA.origin).toEqual([-6, 6, 0]);
    expect(classicRA.uv.front.w).toBe(4);

    // Slim Right Arm (3px width)
    const slimRA = slimParts.find((p) => p.name === 'rightArm')!;
    expect(slimRA.size).toEqual([3, 12, 4]);
    expect(slimRA.origin).toEqual([-5.5, 6, 0]);
    expect(slimRA.uv.front.w).toBe(3);

    // Classic Left Arm
    const classicLA = classicParts.find((p) => p.name === 'leftArm')!;
    expect(classicLA.size).toEqual([4, 12, 4]);
    expect(classicLA.origin).toEqual([6, 6, 0]);

    // Slim Left Arm
    const slimLA = slimParts.find((p) => p.name === 'leftArm')!;
    expect(slimLA.size).toEqual([3, 12, 4]);
    expect(slimLA.origin).toEqual([5.5, 6, 0]);
  });

  it('provides outer layer definitions expanded by 0.5 units for hat, jacket, sleeves, and pants', () => {
    const parts = getCharacterPartDefinitions('default');
    const outerParts = parts.filter((p) => p.isOuter);

    expect(outerParts.length).toBe(6); // hat, jacket, rightSleeve, leftSleeve, rightPants, leftPants

    const hat = parts.find((p) => p.name === 'hat')!;
    expect(hat.isOuter).toBe(true);
    expect(hat.size).toEqual([8.5, 8.5, 8.5]);
    expect(hat.uv.front).toEqual({ x: 40, y: 8, w: 8, h: 8 });

    const jacket = parts.find((p) => p.name === 'jacket')!;
    expect(jacket.isOuter).toBe(true);
    expect(jacket.size).toEqual([8.5, 12.5, 4.5]);
    expect(jacket.uv.front).toEqual({ x: 20, y: 36, w: 8, h: 12 });
  });

  it('correctly parses cape UV layout and 3D bounds', () => {
    const cape = getCapePartDefinition();
    expect(cape.name).toBe('cape');
    expect(cape.size).toEqual([10, 16, 1]);
    expect(cape.origin).toEqual([0, 4, -2.5]);
    expect(cape.uv.back).toEqual({ x: 1, y: 1, w: 10, h: 16 });
    expect(cape.uv.front).toEqual({ x: 12, y: 1, w: 10, h: 16 });
  });

  it('builds valid interleaved vertex buffer with positions, normals, and UVs', () => {
    const { buffer, vertexCount } = buildCharacterModelBuffer('default');

    expect(buffer).toBeInstanceOf(Float32Array);
    expect(vertexCount).toBe(12 * 36); // 12 parts * 36 vertices per box = 432 vertices
    expect(buffer.length).toBe(vertexCount * 8); // 8 floats per vertex [x, y, z, nx, ny, nz, u, v]

    // Verify normals are unit vectors and UV coordinates are within 0..1 range
    for (let i = 0; i < vertexCount; i++) {
      const offset = i * 8;
      const nx = buffer[offset + 3];
      const ny = buffer[offset + 4];
      const nz = buffer[offset + 5];
      const u = buffer[offset + 6];
      const v = buffer[offset + 7];

      const normLen = Math.sqrt(nx * nx + ny * ny + nz * nz);
      expect(normLen).toBeCloseTo(1.0, 4);

      expect(u).toBeGreaterThanOrEqual(0.0);
      expect(u).toBeLessThanOrEqual(1.0);
      expect(v).toBeGreaterThanOrEqual(0.0);
      expect(v).toBeLessThanOrEqual(1.0);
    }
  });

  it('builds cape mesh buffer matching 64x32 texture coordinates', () => {
    const { buffer, vertexCount } = buildCapeModelBuffer();
    expect(vertexCount).toBe(36);
    expect(buffer.length).toBe(36 * 8);

    // Verify all UVs normalized to 64x32
    for (let i = 0; i < vertexCount; i++) {
      const u = buffer[i * 8 + 6];
      const v = buffer[i * 8 + 7];
      expect(u).toBeGreaterThanOrEqual(0.0);
      expect(u).toBeLessThanOrEqual(1.0);
      expect(v).toBeGreaterThanOrEqual(0.0);
      expect(v).toBeLessThanOrEqual(1.0);
    }
  });
});

describe('WebGL Fallback & Support Checks', () => {
  it('returns false when canvas has no WebGL support in test jsdom environment', () => {
    const canvas = document.createElement('canvas');
    // In jsdom without webgl context, getContext('webgl') returns null
    const supported = isWebGLSupported(canvas);
    expect(supported).toBe(false);
  });

  it('returns true when canvas provides a valid WebGL context', () => {
    const mockCanvas = {
      getContext: vi.fn((type: string) => {
        if (type === 'webgl' || type === 'webgl2') {
          return {
            createShader: vi.fn(),
            createProgram: vi.fn(),
          };
        }
        return null;
      }),
    } as unknown as HTMLCanvasElement;

    expect(isWebGLSupported(mockCanvas)).toBe(true);
  });

  it('safely handles exceptions during WebGL probe', () => {
    const brokenCanvas = {
      getContext: vi.fn(() => {
        throw new Error('GPU process crashed');
      }),
    } as unknown as HTMLCanvasElement;

    expect(isWebGLSupported(brokenCanvas)).toBe(false);
  });
});

describe('Default Skin & Cape Texture Generators', () => {
  it('generates standard 64x64 skin canvas for default Steve and Alex', () => {
    const steve = createDefaultSkinCanvas('default');
    expect(steve.width).toBe(64);
    expect(steve.height).toBe(64);

    const alex = createDefaultSkinCanvas('slim');
    expect(alex.width).toBe(64);
    expect(alex.height).toBe(64);
  });
});

describe('3D Matrix Math (Mat4)', () => {
  it('computes identity matrix', () => {
    const m = Mat4.identity();
    expect(m[0]).toBe(1);
    expect(m[5]).toBe(1);
    expect(m[10]).toBe(1);
    expect(m[15]).toBe(1);
  });

  it('translates and multiplies matrices correctly', () => {
    const ident = Mat4.identity();
    const translated = Mat4.translate(ident, [10, -5, 20]);
    expect(translated[12]).toBe(10);
    expect(translated[13]).toBe(-5);
    expect(translated[14]).toBe(20);

    const rotY = Mat4.rotateY(ident, Math.PI / 2);
    const combined = Mat4.multiply(translated, rotY);
    expect(combined).toBeInstanceOf(Float32Array);
  });

  it('rotates around Z axis and around pivot point', () => {
    const ident = Mat4.identity();
    const rotZ = Mat4.rotateZ(ident, Math.PI / 4);
    expect(rotZ).toBeInstanceOf(Float32Array);

    const pivoted = Mat4.rotateAroundPivot(ident, [0, 12, 0], Math.PI / 6, 0, 0);
    expect(pivoted).toBeInstanceOf(Float32Array);
    // When rx = ry = rz = 0, should return original matrix
    expect(Mat4.rotateAroundPivot(ident, [0, 12, 0], 0, 0, 0)).toBe(ident);
  });
});

describe('Character Limb Geometries & Hierarchical Animation', () => {
  it('builds 6 separate limb groups with correct joint pivots for default model', () => {
    const limbs = buildCharacterLimbGeometries('default');
    expect(limbs.length).toBe(6);

    const names = limbs.map((l) => l.name);
    expect(names).toEqual(['head', 'torso', 'rightArm', 'leftArm', 'rightLeg', 'leftLeg']);

    const head = limbs.find((l) => l.name === 'head')!;
    expect(head.pivot).toEqual([0, 12, 0]); // neck
    expect(head.vertexCount).toBe(72); // head (36) + hat (36)

    const rightArm = limbs.find((l) => l.name === 'rightArm')!;
    expect(rightArm.pivot).toEqual([-6, 12, 0]); // classic shoulder

    const leftArm = limbs.find((l) => l.name === 'leftArm')!;
    expect(leftArm.pivot).toEqual([6, 12, 0]);

    const rightLeg = limbs.find((l) => l.name === 'rightLeg')!;
    expect(rightLeg.pivot).toEqual([-2, 0, 0]); // hip
  });

  it('adjusts shoulder pivots for slim (Alex) model', () => {
    const limbs = buildCharacterLimbGeometries('slim');
    const rightArm = limbs.find((l) => l.name === 'rightArm')!;
    const leftArm = limbs.find((l) => l.name === 'leftArm')!;

    expect(rightArm.pivot).toEqual([-5.5, 12, 0]);
    expect(leftArm.pivot).toEqual([5.5, 12, 0]);
  });
});

describe('Skin3DRenderer Animation Lifecycle & Pause Tests', () => {
  function createMockWebGLCanvas() {
    const mockGL = {
      createShader: vi.fn(() => ({})),
      shaderSource: vi.fn(),
      compileShader: vi.fn(),
      getShaderParameter: vi.fn(() => true),
      createProgram: vi.fn(() => ({})),
      attachShader: vi.fn(),
      linkProgram: vi.fn(),
      getProgramParameter: vi.fn(() => true),
      enable: vi.fn(),
      depthFunc: vi.fn(),
      blendFunc: vi.fn(),
      createBuffer: vi.fn(() => ({})),
      bindBuffer: vi.fn(),
      bufferData: vi.fn(),
      createTexture: vi.fn(() => ({})),
      bindTexture: vi.fn(),
      pixelStorei: vi.fn(),
      texImage2D: vi.fn(),
      texParameteri: vi.fn(),
      clearColor: vi.fn(),
      clear: vi.fn(),
      useProgram: vi.fn(),
      getUniformLocation: vi.fn(() => ({})),
      uniform3f: vi.fn(),
      uniform1i: vi.fn(),
      uniform1f: vi.fn(),
      uniformMatrix4fv: vi.fn(),
      getAttribLocation: vi.fn(() => 0),
      enableVertexAttribArray: vi.fn(),
      vertexAttribPointer: vi.fn(),
      drawArrays: vi.fn(),
      viewport: vi.fn(),
      deleteBuffer: vi.fn(),
      deleteTexture: vi.fn(),
      deleteProgram: vi.fn(),
      activeTexture: vi.fn(),
      DEPTH_TEST: 0x0b71,
      LEQUAL: 0x0203,
      BLEND: 0x0be2,
      SRC_ALPHA: 0x0302,
      ONE_MINUS_SRC_ALPHA: 0x0303,
      ARRAY_BUFFER: 0x8892,
      STATIC_DRAW: 0x88e4,
      TEXTURE_2D: 0x0de1,
      COLOR_BUFFER_BIT: 0x4000,
      DEPTH_BUFFER_BIT: 0x0100,
      FLOAT: 0x1406,
      TRIANGLES: 0x0004,
      TEXTURE0: 0x84c0,
      TEXTURE1: 0x84c1,
      TEXTURE_MAG_FILTER: 0x2800,
      TEXTURE_MIN_FILTER: 0x2801,
      TEXTURE_WRAP_S: 0x2802,
      TEXTURE_WRAP_T: 0x2803,
      NEAREST: 0x2600,
      CLAMP_TO_EDGE: 0x812f,
      UNPACK_PREMULTIPLY_ALPHA_WEBGL: 0x9241,
      RGBA: 0x1908,
      UNSIGNED_BYTE: 0x1401,
      VERTEX_SHADER: 0x8b31,
      FRAGMENT_SHADER: 0x8b30,
      LINK_STATUS: 0x8b82,
      COMPILE_STATUS: 0x8b81,
    };

    const canvas = {
      getContext: vi.fn((type: string) => {
        if (type === 'webgl' || type === 'webgl2') return mockGL;
        return null;
      }),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      width: 280,
      height: 320,
      clientWidth: 280,
      clientHeight: 320,
    } as unknown as HTMLCanvasElement;

    return { canvas, mockGL };
  }

  it('tracks frame count and pauses when pause() is invoked', () => {
    const { canvas } = createMockWebGLCanvas();
    const renderer = new Skin3DRenderer({ canvas });

    expect(renderer.getFrameCount()).toBe(0);
    expect(renderer.frameCount).toBe(0);
    expect(renderer.isRenderPaused()).toBe(false);

    // Render 3 explicit frames
    renderer.render();
    renderer.render();
    renderer.render();

    expect(renderer.getFrameCount()).toBe(3);
    expect(renderer.frameCount).toBe(3);

    // Pause rendering
    renderer.pause();
    expect(renderer.isRenderPaused()).toBe(true);
    expect(renderer.isPaused).toBe(true);

    const frameCountAtPause = renderer.getFrameCount();

    // While paused, resume sets it back to active
    renderer.resume();
    expect(renderer.isRenderPaused()).toBe(false);

    renderer.render();
    expect(renderer.getFrameCount()).toBe(frameCountAtPause + 1);

    renderer.destroy();
    expect(renderer.isRenderPaused()).toBe(true);
  });

  it('toggles between idle and walking animation modes', () => {
    const { canvas } = createMockWebGLCanvas();
    const renderer = new Skin3DRenderer({ canvas, animation: 'idle' });

    expect(renderer.animation).toBe('idle');
    expect(renderer.getAnimation()).toBe('idle');

    renderer.setAnimation('walking');
    expect(renderer.animation).toBe('walking');
    expect(renderer.getAnimation()).toBe('walking');

    renderer.render(); // Should render walking poses without throwing
    expect(renderer.getFrameCount()).toBe(1);

    renderer.destroy();
  });

  it('automatically pauses rendering when game is active or window suspended', () => {
    const { canvas } = createMockWebGLCanvas();
    const renderer = new Skin3DRenderer({ canvas });

    expect(renderer.isRenderPaused()).toBe(false);

    // Simulate Minecraft game launch
    setGameRunning(true);
    expect(renderer.isRenderPaused()).toBe(true);

    // Simulate Minecraft game exit
    setGameRunning(false);
    expect(renderer.isRenderPaused()).toBe(false);

    renderer.destroy();
  });
});

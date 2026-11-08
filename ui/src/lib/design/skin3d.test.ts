import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  makeBoxUV,
  getCharacterPartDefinitions,
  getCapePartDefinition,
  buildBoxVertices,
  buildCharacterModelBuffer,
  buildCapeModelBuffer,
  isWebGLSupported,
  createDefaultSkinCanvas,
  createAuraCapeCanvas,
  Mat4,
} from './skin3d';

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

  it('generates 64x32 cape canvas for Aura Cyan and Aura Silver', () => {
    const cyan = createAuraCapeCanvas('cyan');
    expect(cyan.width).toBe(64);
    expect(cyan.height).toBe(32);

    const silver = createAuraCapeCanvas('silver');
    expect(silver.width).toBe(64);
    expect(silver.height).toBe(32);
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
});

/**
 * The look shown when no skin of your own is set: a wide (Steve-like) or slim
 * (Alex-like) texture drawn in code on the standard 64×64 layout. Not a Mojang
 * asset. Every face is laid out by the standard cuboid unwrap (top, bottom,
 * right, front, left, back), and each pixel gets a small fixed variation of
 * brightness so the surfaces read like a game texture instead of flat paint.
 */

export type ModelType = 'default' | 'slim';

type RGB = [number, number, number];

function hex(c: string): RGB {
  const n = parseInt(c.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

/** Fixed per-pixel noise in [-1, 1] (no Math.random: the same skin every time). */
function noise(x: number, y: number): number {
  let h = (x * 374761393 + y * 668265263) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return (((h ^ (h >>> 16)) >>> 0) / 4294967295) * 2 - 1;
}

export function createDefaultSkinCanvas(model: ModelType): HTMLCanvasElement {
  const canvas = document.createElement('canvas');
  canvas.width = 64;
  canvas.height = 64;
  let ctx: CanvasRenderingContext2D | null = null;
  try {
    ctx = canvas.getContext('2d');
  } catch {
    // No canvas (tests without a 2D context): an empty texture.
  }
  if (!ctx || typeof ctx.createImageData !== 'function') return canvas;
  const img = ctx.createImageData(64, 64);
  if (!img?.data) return canvas;
  const slim = model === 'slim';
  const aw = slim ? 3 : 4;

  const C = {
    skin: hex(slim ? '#e8b48c' : '#b0805a'),
    skinDark: hex(slim ? '#d39c75' : '#946644'),
    hair: hex(slim ? '#c8642a' : '#4a3020'),
    hairDark: hex(slim ? '#a54f1f' : '#3a2416'),
    eyeWhite: hex('#f4f4f4'),
    eye: hex(slim ? '#2f7a55' : '#4a5bb8'),
    mouth: hex(slim ? '#b57a5c' : '#6b412a'),
    shirt: hex(slim ? '#5f9a35' : '#00a6a6'),
    shirtDark: hex(slim ? '#4b7d29' : '#008787'),
    pants: hex(slim ? '#5a4634' : '#2b3a98'),
    pantsDark: hex(slim ? '#46362a' : '#212d7a'),
    belt: hex(slim ? '#3d2f23' : '#1d266a'),
    shoes: hex(slim ? '#3a3026' : '#5c5c5c'),
  };

  const put = (x: number, y: number, c: RGB, amt = 0.07) => {
    if (x < 0 || y < 0 || x >= 64 || y >= 64) return;
    const k = 1 + noise(x, y) * amt;
    const i = (y * 64 + x) * 4;
    img.data[i] = Math.max(0, Math.min(255, Math.round(c[0] * k)));
    img.data[i + 1] = Math.max(0, Math.min(255, Math.round(c[1] * k)));
    img.data[i + 2] = Math.max(0, Math.min(255, Math.round(c[2] * k)));
    img.data[i + 3] = 255;
  };
  const rect = (x: number, y: number, w: number, h: number, c: RGB | ((px: number, py: number) => RGB), amt?: number) => {
    for (let py = 0; py < h; py++) for (let px = 0; px < w; px++) put(x + px, y + py, typeof c === 'function' ? c(px, py) : c, amt);
  };

  /**
   * Paints a cuboid at texture (u, v) of size w×h×d. `side(face, px, py, fw)` gives
   * the colour of a pixel of the right/front/left/back faces (py from the top).
   */
  const box = (
    u: number,
    v: number,
    w: number,
    h: number,
    d: number,
    top: RGB,
    bottom: RGB,
    side: (face: 'right' | 'front' | 'left' | 'back', px: number, py: number, fw: number) => RGB
  ) => {
    rect(u + d, v, w, d, top);
    rect(u + d + w, v, w, d, bottom);
    rect(u, v + d, d, h, (px, py) => side('right', px, py, d));
    rect(u + d, v + d, w, h, (px, py) => side('front', px, py, w));
    rect(u + d + w, v + d, d, h, (px, py) => side('left', px, py, d));
    rect(u + d + w + d, v + d, w, h, (px, py) => side('back', px, py, w));
  };

  // Head: hair on top, on the back and down the sides; face in front; chin below.
  box(0, 0, 8, 8, 8, C.hair, C.skinDark, (face, px, py) => {
    if (face === 'back') return py < 7 ? C.hair : C.hairDark;
    if (face === 'front') return py < 2 ? (py === 1 && (px === 0 || px === 7) ? C.hairDark : C.hair) : C.skin;
    // Sides: hair to the ear, then skin; a darker sideburn at the front edge.
    const front = face === 'right' ? px === 7 : px === 0;
    if (py < 3 || (py < 5 && !front)) return C.hair;
    return front && py < 6 ? C.hairDark : C.skin;
  });
  // Face details on the front (8..16, 8..16).
  // Eyes: white outside, iris inside (9 10 | 13 14).
  put(9, 12, C.eyeWhite, 0.02);
  put(10, 12, C.eye, 0.02);
  put(13, 12, C.eye, 0.02);
  put(14, 12, C.eyeWhite, 0.02);
  rect(11, 14, 2, 1, C.mouth, 0.02);
  put(12, 13, C.skinDark);
  put(11, 13, C.skinDark);

  // Body: shirt with a darker collar and a belt line at the bottom.
  box(16, 16, 8, 12, 4, C.shirt, C.pants, (face, px, py, fw) => {
    if (py >= 11) return C.belt;
    if (face === 'front' && py === 0 && px >= 2 && px < fw - 2) return C.skin;
    return py === 0 ? C.shirtDark : C.shirt;
  });

  // Arms: short sleeve, then skin; the hand is a touch darker.
  const arm = (u: number, v: number) =>
    box(u, v, aw, 12, 4, C.shirt, C.skinDark, (_face, _px, py) => (py < 4 ? (py === 3 ? C.shirtDark : C.shirt) : py >= 11 ? C.skinDark : C.skin));
  arm(40, 16);
  arm(32, 48);

  // Legs: trousers, a seam shade at the knee, shoes.
  const leg = (u: number, v: number) =>
    box(u, v, 4, 12, 4, C.pants, C.shoes, (_face, _px, py) => (py >= 10 ? C.shoes : py === 6 ? C.pantsDark : C.pants));
  leg(0, 16);
  leg(16, 48);

  ctx.putImageData(img, 0, 0);
  return canvas;
}

/**
 * Minecraft skin model built from CSS 3D boxes (reference: Гардероб › SkinStage).
 * Every face is a canvas cut from the real 64×64 texture by the standard skin
 * layout: head + hat, body + jacket, arms + sleeves (wide 4 px or slim 3 px),
 * legs + pants. A cape (64×32 layout, any multiple) hangs behind the body.
 *
 * Face names follow the skin layout: "right" is the character's right side, which
 * faces the viewer's left when the model looks at you.
 */

export type Face = 'top' | 'bottom' | 'right' | 'front' | 'left' | 'back';
/** [x, y, width, height] on the 64×64 texture. */
export type Rect = readonly [number, number, number, number];
export type BoxUV = Readonly<Record<Face, Rect>>;

/** UV of a box whose texture starts at (u, v) with size w×h×d (standard Minecraft cuboid unwrap). */
export function boxUV(u: number, v: number, w: number, h: number, d: number): BoxUV {
  return {
    top: [u + d, v, w, d],
    bottom: [u + d + w, v, w, d],
    right: [u, v + d, d, h],
    front: [u + d, v + d, w, h],
    left: [u + d + w, v + d, d, h],
    back: [u + d + w + d, v + d, w, h],
  };
}

export interface Part {
  name: 'head' | 'body' | 'armR' | 'armL' | 'legR' | 'legL';
  /** Size in skin pixels: width, height, depth. */
  size: [number, number, number];
  base: BoxUV;
  overlay: BoxUV;
}

/** The six parts with base and outer layers for a wide (4 px) or slim (3 px) arm model. */
export function skinParts(slim: boolean): Part[] {
  const aw = slim ? 3 : 4;
  return [
    { name: 'head', size: [8, 8, 8], base: boxUV(0, 0, 8, 8, 8), overlay: boxUV(32, 0, 8, 8, 8) },
    { name: 'body', size: [8, 12, 4], base: boxUV(16, 16, 8, 12, 4), overlay: boxUV(16, 32, 8, 12, 4) },
    { name: 'armR', size: [aw, 12, 4], base: boxUV(40, 16, aw, 12, 4), overlay: boxUV(40, 32, aw, 12, 4) },
    { name: 'armL', size: [aw, 12, 4], base: boxUV(32, 48, aw, 12, 4), overlay: boxUV(48, 48, aw, 12, 4) },
    { name: 'legR', size: [4, 12, 4], base: boxUV(0, 16, 4, 12, 4), overlay: boxUV(0, 32, 4, 12, 4) },
    { name: 'legL', size: [4, 12, 4], base: boxUV(16, 48, 4, 12, 4), overlay: boxUV(0, 48, 4, 12, 4) },
  ];
}

/** Cape on a 64×32 texture: the outer side (seen from behind) is at (1,1), the side facing the body at (12,1). */
export const CAPE_UV: BoxUV = {
  top: [1, 0, 10, 1],
  bottom: [11, 0, 10, 1],
  right: [0, 1, 1, 16],
  front: [12, 1, 10, 16],
  left: [11, 1, 1, 16],
  back: [1, 1, 10, 16],
};

/** CSS pixels per skin pixel (reference: U = 11). */
export const U = 11;

/** A 64×64 canvas from a skin image; 64×32 legacy skins get their left arm and leg mirrored in, as Minecraft does. */
export function toSkinCanvas(img: CanvasImageSource & { width: number; height: number }): HTMLCanvasElement {
  const cv = document.createElement('canvas');
  cv.width = 64;
  cv.height = 64;
  const c = cv.getContext('2d');
  if (!c) return cv;
  c.imageSmoothingEnabled = false;
  c.drawImage(img, 0, 0);
  if (img.height === 32) {
    c.drawImage(img, 0, 16, 16, 16, 16, 48, 16, 16);
    c.drawImage(img, 40, 16, 16, 16, 32, 48, 16, 16);
  }
  return cv;
}

export function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('image'));
    img.src = src;
  });
}

/** Front view 16×32 (head + hat, body + jacket, arms, legs) for skin cards (reference: thumb). */
export function drawThumb(cv: HTMLCanvasElement, tex: CanvasImageSource, slim: boolean): void {
  cv.width = 16;
  cv.height = 32;
  const c = cv.getContext('2d');
  if (!c) return;
  c.imageSmoothingEnabled = false;
  c.clearRect(0, 0, 16, 32);
  const parts = skinParts(slim);
  const aw = slim ? 3 : 4;
  const at: Record<Part['name'], [number, number]> = {
    head: [4, 0],
    body: [4, 8],
    armR: [4 - aw, 8],
    armL: [12, 8],
    legR: [4, 20],
    legL: [8, 20],
  };
  for (const layer of ['base', 'overlay'] as const) {
    for (const p of parts) {
      const [x, y, w, h] = p[layer].front;
      c.drawImage(tex, x, y, w, h, at[p.name][0], at[p.name][1], w, h);
    }
  }
}

const FACE_TRANSFORM = (f: Face, w: number, h: number, d: number): string =>
  ({
    front: `translateZ(${(d * U) / 2}px)`,
    back: `rotateY(180deg) translateZ(${(d * U) / 2}px)`,
    right: `rotateY(-90deg) translateZ(${(w * U) / 2}px)`,
    left: `rotateY(90deg) translateZ(${(w * U) / 2}px)`,
    top: `rotateX(90deg) translateZ(${(h * U) / 2}px)`,
    bottom: `rotateX(-90deg) translateZ(${(h * U) / 2}px)`,
  })[f];

const FACE_SIZE = (f: Face, w: number, h: number, d: number): [number, number] =>
  f === 'front' || f === 'back' ? [w, h] : f === 'right' || f === 'left' ? [d, h] : [w, d];

/**
 * One cuboid of canvases. `scale` maps texture pixels (e.g. 2 for a 128×64 cape).
 * Bottom faces are flipped vertically, as Minecraft stores them.
 */
export function box(uv: BoxUV, size: [number, number, number], tex: CanvasImageSource, extra = '', scale = 1): HTMLDivElement {
  const [w, h, d] = size;
  const el = document.createElement('div');
  el.className = `box ${extra}`.trim();
  for (const f of ['front', 'back', 'right', 'left', 'top', 'bottom'] as Face[]) {
    const [fw, fh] = FACE_SIZE(f, w, h, d);
    const [x, y, sw, sh] = uv[f];
    const cv = document.createElement('canvas');
    cv.width = Math.max(1, Math.round(sw * scale));
    cv.height = Math.max(1, Math.round(sh * scale));
    cv.className = 'face';
    cv.dataset.face = f;
    const ctx = cv.getContext('2d');
    if (ctx) {
      ctx.imageSmoothingEnabled = false;
      if (f === 'bottom') {
        ctx.translate(0, cv.height);
        ctx.scale(1, -1);
      }
      ctx.drawImage(tex, x * scale, y * scale, sw * scale, sh * scale, 0, 0, cv.width, cv.height);
    }
    cv.style.width = `${fw * U}px`;
    cv.style.height = `${fh * U}px`;
    cv.style.marginLeft = `${(-fw * U) / 2}px`;
    cv.style.marginTop = `${(-fh * U) / 2}px`;
    cv.style.transform = FACE_TRANSFORM(f, w, h, d);
    el.appendChild(cv);
  }
  return el;
}

/** A joint at (px,py,pz) whose child hangs at (cx,cy,cz) from it; poses rotate `.swing`. */
function joint(name: string, p: [number, number, number], child: HTMLElement, c: [number, number, number]): HTMLDivElement {
  const j = document.createElement('div');
  j.className = `joint j-${name}`;
  j.style.transform = `translate3d(${p[0] * U}px, ${p[1] * U}px, ${p[2] * U}px)`;
  const swing = document.createElement('div');
  swing.className = 'swing';
  child.style.transform = `translate3d(${c[0] * U}px, ${c[1] * U}px, ${c[2] * U}px)`;
  swing.appendChild(child);
  j.appendChild(swing);
  return j;
}

/** Outer layers are half a pixel larger on each side, as in the game. */
const OVER = 0.5;

function layered(part: Part, tex: CanvasImageSource): HTMLDivElement {
  const g = document.createElement('div');
  g.className = 'box-group';
  g.appendChild(box(part.base, part.size, tex));
  const [w, h, d] = part.size;
  g.appendChild(box(part.overlay, [w + OVER, h + OVER, d + OVER], tex, part.name === 'head' ? 'hat overlay' : 'overlay'));
  return g;
}

export interface CapeTexture {
  canvas: CanvasImageSource;
  /** Texture width / 64 (1 for 64×32, 2 for 128×64…). */
  scale: number;
}

/** Fill `rig` with the model. Positions follow the reference rig (height centred at 16 px). */
export function buildRig(rig: HTMLElement, tex: CanvasImageSource, slim: boolean, cape: CapeTexture | null): void {
  rig.replaceChildren();
  const aw = slim ? 3 : 4;
  const off = -16;
  const parts = Object.fromEntries(skinParts(slim).map((p) => [p.name, p])) as Record<Part['name'], Part>;
  rig.appendChild(joint('head', [0, off + 8, 0], layered(parts.head, tex), [0, -4, 0]));
  rig.appendChild(joint('body', [0, off + 8, 0], layered(parts.body, tex), [0, 6, 0]));
  rig.appendChild(joint('armR', [-(4 + aw / 2), off + 8, 0], layered(parts.armR, tex), [0, 5, 0]));
  rig.appendChild(joint('armL', [4 + aw / 2, off + 8, 0], layered(parts.armL, tex), [0, 5, 0]));
  rig.appendChild(joint('legR', [-2, off + 20, 0], layered(parts.legR, tex), [0, 6, 0]));
  rig.appendChild(joint('legL', [2, off + 20, 0], layered(parts.legL, tex), [0, 6, 0]));
  if (cape) {
    rig.appendChild(joint('cape', [0, off + 8, -2.6], box(CAPE_UV, [10, 16, 0.6], cape.canvas, 'cape', cape.scale), [0, 8, 0]));
  }
}

export const NICK_RE = /^[A-Za-z0-9_]{3,16}$/;

/** Is this PNG a skin (64×64 or legacy 64×32)? */
export function isSkinSize(w: number, h: number): boolean {
  return w === 64 && (h === 64 || h === 32);
}

/** Is this PNG a cape in the standard 64×32 layout or a whole multiple of it (128×64, 256×128…)? */
export function isCapeSize(w: number, h: number): boolean {
  return w >= 64 && w % 64 === 0 && h * 2 === w;
}

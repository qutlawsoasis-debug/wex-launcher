/**
 * The look shown when no skin of your own is set: a simple wide (Steve-like) or
 * slim (Alex-like) texture drawn in code on the standard 64×64 layout. Not a
 * Mojang asset.
 */

export type ModelType = 'default' | 'slim';

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

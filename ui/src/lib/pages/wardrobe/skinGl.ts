/**
 * The wardrobe model rendered with WebGL (skinview3d, MIT): real Minecraft
 * proportions, both skin layers, slim or wide arms, cape, perspective and light.
 * The page keeps its own turn/pose state and drives the viewer; the CSS model
 * (skinRig.ts) stays as the fallback when WebGL is unavailable.
 * skinview3d (with three.js) is loaded only when the wardrobe opens.
 */

import type { SkinViewer } from 'skinview3d';

export type Pose = 'idle' | 'walk' | 'run' | 'wave';

export function webglAvailable(): boolean {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
  } catch {
    return false;
  }
}

type Lib = typeof import('skinview3d');

export class SkinStage {
  private constructor(
    private lib: Lib,
    readonly viewer: SkinViewer
  ) {}

  private pose: Pose = 'idle';
  private reduced = false;

  static async create(canvas: HTMLCanvasElement, width: number, height: number): Promise<SkinStage> {
    const lib = await import('skinview3d');
    const viewer = new lib.SkinViewer({
      canvas,
      width,
      height,
      pixelRatio: 'match-device',
      enableControls: false,
      fov: 30,
      zoom: 0.62,
      preserveDrawingBuffer: true,
    });
    viewer.background = null;
    viewer.globalLight.intensity = 3;
    viewer.cameraLight.intensity = 0.6;
    // Seen a little from above, as the CSS model was.
    viewer.playerWrapper.rotation.x = 0.1;
    const stage = new SkinStage(lib, viewer);
    stage.setPose('idle');
    return stage;
  }

  setSkin(source: HTMLCanvasElement | HTMLImageElement, slim: boolean): void {
    this.viewer.loadSkin(source, { model: slim ? 'slim' : 'default' });
  }

  setCape(source: CanvasImageSource | null): void {
    if (source) this.viewer.loadCape(source as HTMLCanvasElement, { backEquipment: 'cape' });
    else this.viewer.loadCape(null);
  }

  private target = 0;
  private raf = 0;

  /**
   * Degrees; positive turns the model to the viewer's right, like the CSS model.
   * `instant` (drag, auto-turn) follows at once; a button turn eases over about
   * 0.3 s, frame-rate independent (none under Reduce Motion).
   */
  setTurn(deg: number, instant: boolean): void {
    this.target = (deg * Math.PI) / 180;
    const wrap = this.viewer.playerWrapper;
    if (instant || this.reduced) {
      cancelAnimationFrame(this.raf);
      this.raf = 0;
      wrap.rotation.y = this.target;
      return;
    }
    if (this.raf) return;
    let last = performance.now();
    const step = (now: number) => {
      const k = 1 - Math.exp(-(now - last) / 90);
      last = now;
      wrap.rotation.y += (this.target - wrap.rotation.y) * k;
      if (Math.abs(this.target - wrap.rotation.y) < 0.001) {
        wrap.rotation.y = this.target;
        this.raf = 0;
        return;
      }
      this.raf = requestAnimationFrame(step);
    };
    this.raf = requestAnimationFrame(step);
  }

  /** The nickname above the head, in the app's font (not the game's). */
  setNick(name: string, font: string): void {
    this.viewer.nameTag = name
      ? new this.lib.NameTagObject(name, {
          font: `600 48px ${font}`,
          textStyle: '#fff',
          backgroundStyle: 'rgba(0,0,0,0.38)',
          repaintAfterLoaded: true,
        })
      : null;
  }

  setPose(pose: Pose): void {
    this.pose = pose;
    const L = this.lib;
    const anim =
      pose === 'walk' ? new L.WalkingAnimation() : pose === 'run' ? new L.RunningAnimation() : pose === 'wave' ? new L.WaveAnimation() : new L.IdleAnimation();
    anim.paused = this.reduced;
    this.viewer.animation = anim;
  }

  /** Reduce Motion: the pose holds still (HIG › Motion). */
  setReducedMotion(on: boolean): void {
    this.reduced = on;
    if (this.viewer.animation) this.viewer.animation.paused = on;
  }

  /** Stops drawing while the launcher idles or the page is hidden. */
  setPaused(on: boolean): void {
    this.viewer.renderPaused = on;
  }

  resize(width: number, height: number): void {
    this.viewer.setSize(width, height);
  }

  get currentPose(): Pose {
    return this.pose;
  }

  dispose(): void {
    cancelAnimationFrame(this.raf);
    this.viewer.dispose();
  }
}

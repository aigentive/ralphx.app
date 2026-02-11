import { Application, Container, Graphics } from "pixi.js";

export interface RendererFrameContext {
  dt: number;
  now: number;
  width: number;
  height: number;
  bgLayer: Graphics;
  worldLayer: Graphics;
  fxLayer: Graphics;
}

export interface RendererBridgeOptions {
  host: HTMLDivElement;
  getSize: () => { width: number; height: number };
  onResize?: (size: { width: number; height: number }) => void;
  onFrame: (ctx: RendererFrameContext) => void;
}

export class RendererBridge {
  private app: Application | null = null;
  private bgLayer: Graphics | null = null;
  private worldLayer: Graphics | null = null;
  private fxLayer: Graphics | null = null;
  private cleanupResize: (() => void) | null = null;

  async start(options: RendererBridgeOptions): Promise<void> {
    const app = new Application();
    const size = options.getSize();
    await app.init({
      width: size.width,
      height: size.height,
      antialias: true,
      backgroundAlpha: 0,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
    });

    this.app = app;
    const canvas = app.canvas;
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    options.host.appendChild(canvas);

    const root = new Container();
    const bgLayer = new Graphics();
    const worldLayer = new Graphics();
    const fxLayer = new Graphics();
    root.addChild(bgLayer, worldLayer, fxLayer);
    app.stage.addChild(root);

    this.bgLayer = bgLayer;
    this.worldLayer = worldLayer;
    this.fxLayer = fxLayer;

    const resize = () => {
      if (!this.app) return;
      const nextSize = options.getSize();
      this.app.renderer.resize(nextSize.width, nextSize.height);
      options.onResize?.(nextSize);
    };

    window.addEventListener("resize", resize);
    this.cleanupResize = () => window.removeEventListener("resize", resize);

    app.ticker.add(() => {
      if (!this.app || !this.bgLayer || !this.worldLayer || !this.fxLayer) return;
      const s = options.getSize();
      options.onFrame({
        dt: Math.min(0.05, this.app.ticker.deltaMS / 1000),
        now: Date.now(),
        width: s.width,
        height: s.height,
        bgLayer: this.bgLayer,
        worldLayer: this.worldLayer,
        fxLayer: this.fxLayer,
      });
    });

    resize();
  }

  destroy(host: HTMLDivElement): void {
    if (this.cleanupResize) {
      this.cleanupResize();
      this.cleanupResize = null;
    }
    if (this.app) {
      this.app.destroy(true);
      this.app = null;
    }
    this.bgLayer = null;
    this.worldLayer = null;
    this.fxLayer = null;
    host.innerHTML = "";
  }
}

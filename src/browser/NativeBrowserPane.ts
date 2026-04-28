// Native browser pane: uses a Tauri child WebviewWindow positioned over a
// placeholder <div> instead of an iframe. This bypasses X-Frame-Options and
// CSP restrictions that limit the iframe-based BrowserPane.
//
// The placeholder element participates in the normal DOM layout (flexbox splits,
// zoom, workspace switching) and a ResizeObserver keeps the native webview
// aligned with it.

import { getCurrentWindow } from "@tauri-apps/api/window";
import type { Uuid } from "../types";
import type { Pane } from "../layout/Pane";
import { api } from "../ipc/bridge";

export interface NativeBrowserPaneOptions {
  id: Uuid;
  url?: string | null;
  onFocus?: () => void;
}

const DEFAULT_URL = "about:blank";

export class NativeBrowserPane implements Pane {
  readonly id: Uuid;
  readonly element: HTMLElement;
  private url: string;
  private placeholder: HTMLDivElement;
  private resizeObserver: ResizeObserver;
  private spawned = false;
  private repositionTimer: number | null = null;
  private opts: NativeBrowserPaneOptions;

  constructor(opts: NativeBrowserPaneOptions) {
    this.id = opts.id;
    this.opts = opts;
    this.url = opts.url?.trim() || DEFAULT_URL;

    // Outer element — same structure as other panes for uniform handling.
    this.element = document.createElement("div");
    this.element.className = "pane native-browser-pane";
    this.element.tabIndex = 0;
    this.element.dataset.paneId = this.id;

    // Placeholder div that occupies the layout slot. The native webview
    // window is positioned to exactly overlay this element.
    this.placeholder = document.createElement("div");
    this.placeholder.className = "native-browser-pane__placeholder";
    this.placeholder.style.flex = "1 1 auto";
    this.placeholder.style.minHeight = "0";
    this.placeholder.style.minWidth = "0";
    this.element.appendChild(this.placeholder);

    // Track size/position changes and reposition the native webview.
    this.resizeObserver = new ResizeObserver(() => {
      this.scheduleReposition();
    });
    this.resizeObserver.observe(this.placeholder);

    this.element.addEventListener("focusin", () => this.opts.onFocus?.());
    this.element.addEventListener("pointerdown", () => this.focus());
  }

  /// Create the native child webview window.
  async spawn(): Promise<void> {
    if (this.spawned) return;
    const rect = await this.getScreenRect();
    await api.createWebview(
      this.id,
      this.url,
      rect.x,
      rect.y,
      rect.width,
      rect.height,
    );
    this.spawned = true;
  }

  /// Reposition the child webview to match the placeholder's screen position.
  async reposition(): Promise<void> {
    if (!this.spawned) return;
    const rect = await this.getScreenRect();
    await api.resizeWebview(this.id, rect.x, rect.y, rect.width, rect.height);
  }

  /// Navigate the native webview to a new URL.
  async navigate(url: string): Promise<void> {
    this.url = url;
    if (this.spawned) {
      await api.navigateWebview(this.id, url);
    }
  }

  focus(): void {
    this.element.focus({ preventScroll: true });
    this.opts.onFocus?.();
  }

  /// Called on layout changes (split resize, zoom toggle, workspace switch).
  scheduleFit(): void {
    this.scheduleReposition();
  }

  dispose(): void {
    this.resizeObserver.disconnect();
    if (this.repositionTimer !== null) {
      cancelAnimationFrame(this.repositionTimer);
      this.repositionTimer = null;
    }
    if (this.spawned) {
      void api.destroyWebview(this.id).catch(() => {});
      this.spawned = false;
    }
    this.element.remove();
  }

  // ─── Private helpers ───────────────────────────────────────────────

  private scheduleReposition(): void {
    if (this.repositionTimer !== null) return;
    this.repositionTimer = requestAnimationFrame(() => {
      this.repositionTimer = null;
      void this.reposition().catch(() => {});
    });
  }

  /// Convert the placeholder's DOM rect to screen-pixel coordinates by
  /// combining the main window's outer position and scale factor.
  private async getScreenRect(): Promise<{
    x: number;
    y: number;
    width: number;
    height: number;
  }> {
    const win = getCurrentWindow();
    const [pos, scale] = await Promise.all([
      win.outerPosition(),
      win.scaleFactor(),
    ]);
    const domRect = this.placeholder.getBoundingClientRect();

    // `pos` is in physical pixels; DOM rect is in CSS (logical) pixels.
    // Convert DOM coords to physical pixels and add the window's physical
    // position offset.
    //
    // Note: outerPosition gives the top-left of the window *frame* (including
    // title bar on platforms that have one). For a frameless window on Windows
    // with WebView2, outerPosition == the content origin. On decorated windows
    // you'd need innerPosition, but ymux's main window is frameless so
    // outerPosition works.
    const x = pos.x + Math.round(domRect.left * scale);
    const y = pos.y + Math.round(domRect.top * scale);
    const width = Math.round(domRect.width * scale);
    const height = Math.round(domRect.height * scale);

    return { x, y, width, height };
  }
}

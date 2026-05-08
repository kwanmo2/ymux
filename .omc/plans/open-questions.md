# Open Questions

Tracks unresolved questions and deferred decisions across all plans.

## browser-engine-upgrade - 2026-05-05

- [ ] Is there a hard product reason to keep the iframe `BrowserPane.ts` after the embedded variant ships, or is iframe purely a transitional fallback? - Affects whether Phase 0+ should plan deletion of `BrowserPane.ts` in vN+1 or treat it as permanent.
- [ ] Should the URL bar live in the layout (Tauri-side, part of the pane chrome) or inside the embedded webview as a custom protocol-served page? - Affects DPI scaling, theming consistency with terminal panes, and z-order behavior during animation.
- [ ] Devtools UX: in-pane (right-click -> Inspect) or external window? - Tauri default is external window; user may want devtools docked inside the pane for parity with browsers.
- [ ] Per-pane vs. shared web data partitioning. - Each pane gets a fresh ephemeral storage partition (private-mode-by-default), or shared cookies/login state across panes? Privacy vs. convenience tradeoff that needs a product call.
- [x] ~~If Phase 0 spike reveals `add_child` is blocked on one platform, does consensus prefer Option B (platform-direct) or Option C (CEF) as the new lead?~~ — **Resolved iteration 2 (2026-05-05):** Linux-only failure -> Option B Linux + Option A Win/macOS. Windows failure -> halt entirely (primary platform per `Cargo.toml` manifest). macOS-only failure -> reconvene consensus.

## browser-engine-upgrade - 2026-05-05 (iteration 2)

- [ ] Scope confirmation: Is the work scoped to fix drift only (MVF stable-path is sufficient, 1-2 days), or all four defect classes — drift + z-order + virtual-desktop + devtools (full Option A, 8.5-10 days)? — Default assumption is full scope. User confirmation requested before Phase 0 P0-B is even attempted, since the MVF only matters if scope is drift-only.

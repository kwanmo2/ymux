# Plan: Browser Pane Engine Upgrade

**Status:** Iteration 2 — revised after Critic ITERATE feedback (RALPLAN-DR, DELIBERATE mode)
**Owner:** Planner -> Architect -> Critic -> Executor
**Date:** 2026-05-05
**Mode:** DELIBERATE (high-risk: cross-platform engine surface, licensing, binary size, `unstable` Tauri API gate)
**Iteration 2 changes:** C1 (Cargo.toml `unstable` feature + `=2.10.3` pin), C2 (Phase 0 split-criteria spike + decision tree + MVF branch), M1 (pre-mortem Scenario 1b for unstable API), M2 (Comparative Summary annotation), M3 (binary-size baseline capture), M4 (no `.unwrap()` in spike; verified main label = "main"), Q5 pre-decided.

---

## 1. Context

yMux currently ships two browser pane implementations, both flawed for the use case:

- `src/browser/BrowserPane.ts` (iframe) — blocked by X-Frame-Options and CSP `frame-ancestors`/sandbox on most production sites. Cannot navigate to e.g. github.com, twitter, banking, etc.
- `src/browser/NativeBrowserPane.ts` (child `WebviewWindow` overlay) — bypasses iframe restrictions, but renders as a *separate top-level window* that the frontend has to keep aligned to the pane rectangle via 33ms polling -> visible drift on resize, drag, snap, virtual-desktop switch, and z-order glitches with dialogs and devtools.

Backend: `src-tauri/src/webview.rs` exposes `create_webview / destroy_webview / navigate_webview / resize_webview` Tauri commands today, all built on `WebviewWindowBuilder` (= a separate window). Tauri version pinned at `tauri = "2"` (resolved 2.10.3 in lockfile cache), which means the modern `WebviewBuilder` + `Window::add_child(builder, position, size)` multi-webview-per-window API is available. This is the same wry/WebView2/WKWebView/webkit2gtk engine yMux already ships - no new engine, no new license, no binary growth.

Reframing the user's request:
- "iframe is too restrictive" -> already solved by *any* native webview; no engine change needed.
- "overlay polling is fragile" -> an *embedding-model* problem (separate window vs. child webview), not an *engine* problem.

Therefore the primary work is **switch the embedding model from `WebviewWindow` (sibling) to `Webview` as a child of the main window**, not swap the underlying engine. The plan keeps a heavier engine swap (CEF) on the table as Option C for the consensus step, and explicitly invalidates Ultralight, Servo, and "fix Win32 only" alternatives.

### Current pane wiring (verified)

- `src/types.ts`: `PaneKind = "terminal" | "browser" | "native_browser"` already supports a third variant cleanly.
- `src/layout/Pane.ts`: minimal interface (`id, element, focus, scheduleFit, spawn, dispose`) - any new pane class plugs in unchanged.
- `src/workspace/WorkspaceManager.ts:222` `createPane` switches on `spec.pane_kind` - one extra branch covers the new pane.
- `src/ipc/bridge.ts` (153 LOC) is the only place that wraps Tauri `invoke` for browser commands - the place to add the new IPC surface.

---

## 2. Work Objectives

1. Replace the geometry-polled overlay model with a properly parented child webview so the pane behaves like terminal panes do (no drift, correct z-order, focus respects pane focus).
2. Keep using the engine yMux already ships (wry: WebView2 / WKWebView / webkit2gtk). No new engine binary, no new license footprint, no new platform-specific install surface.
3. Land behind a feature flag side-by-side with `BrowserPane` and `NativeBrowserPane` so the migration is reversible per release.
4. Cross-platform from day one (Windows / macOS / Linux). No Windows-only intermediate state shipped.
5. Preserve the `Pane` interface contract; layout/focus/persistence code stays untouched.

---

## 3. Guardrails

### Must Have

- New pane variant `pane_kind: "embedded_browser"` shipped behind a runtime/build feature flag (default OFF) so the existing two implementations remain available for at least one release after merge.
- Cross-platform parity. CI must build successfully on Windows, macOS, and Linux. No `cfg(target_os)` branches in pane bookkeeping logic - only in the Rust IPC layer if absolutely required.
- Pane geometry follows the layout tree without polling. Position/size updates flow `LayoutNode -> WorkspaceManager -> bridge.ts -> Tauri command -> Webview::set_position/set_size` and happen on layout dirty events only.
- Disposal is idempotent and synchronous from the frontend's perspective: `dispose()` removes the child webview and releases the label slot.
- ADR captured in `.omc/plans/adrs/browser-engine.md` after consensus selects a path.

### Must NOT Have

- No raw Win32 `SetParent` / `SetWindowLongPtr` / WS_CHILD bit-flipping in the production path. (Acceptable as a fallback only if the Tauri multi-webview path proves blocked, and then only with documented macOS/Linux equivalents.)
- No new heavy native dependency (CEF, Ultralight, Servo embed) in the chosen Option A path. Those belong to Option C and require explicit consensus sign-off.
- No deletion of `BrowserPane.ts` or `NativeBrowserPane.ts` in this work. Removal is a follow-up after the new path bakes in a release.
- No Tauri major version bump in this plan. Stay on 2.x.
- No changes to terminal pane code paths.

---

## 4. Recommended Option (lead)

**Option A: Tauri 2 multi-webview (`Window::add_child` + `WebviewBuilder`)**

Use `tauri::webview::WebviewBuilder` to construct a child webview parented to the existing main `Window`, positioned and sized by the layout engine. This is the API Tauri 2 added specifically for "multiple webviews inside one window" and is verified present in tauri 2.10.3 (`src/window/mod.rs:1052: pub fn add_child<P: Into<Position>, S: Into<Size>>`).

Why this is the right shape:
- Same engine yMux already ships -> zero binary delta, zero new license obligations, zero platform install delta.
- Native parent/child relationship -> no polling, no z-order glitches, focus and DPI handled by the platform.
- Cross-platform automatically through wry.
- `Webview` exposes `set_position`, `set_size`, `set_focus`, `navigate`, `close` as direct calls - mirrors today's `webview.rs` command shape almost 1:1.
- The Rust changes are local to `webview.rs`; frontend changes are a new `EmbeddedBrowserPane` class plus one extra branch in `WorkspaceManager.createPane`.

Risk that drove this to be the lead, not the only option:
- The `add_child` API is younger than `WebviewWindow` and has known rough edges around devtools, transparency, and rapid create/destroy churn. Mitigated by the feature flag and the side-by-side migration.

---

## 5. Phased Implementation

Six phases, each independently testable. Phase 0 is a spike that gates everything else.

### Phase 0 - Parallel spike: `add_child` vs. stable-API path (1-2 days, gating)

**Minimum Viable Fix (MVF) branch:** Before spending 8.5 days, this plan offers a 1-2 day exit. If user confirmation arrives during Phase 0 that *only the geometry/drift problem matters* (z-order, virtual-desktop migration, and devtools parity are explicitly NOT required), ship the MVF stable-path branch and close. The MVF is structurally insufficient for z-order/virtual-desktop/devtools defects (see Critic verification: `WebviewWindow::parent()` still produces a top-level OS window), so it is only viable if those defects are out of scope.

**Verified pre-conditions (do before writing any spike code):**
- Confirm main window label is `"main"` per `src-tauri/tauri.conf.json:15` (verified) and `capabilities/default.json:5` (verified). If the label has changed, update both spike paths.
- Confirm `tauri = "=2.10.3"` is pinned in `src-tauri/Cargo.toml` *before* compiling either spike path; `add_child` is gated on `feature = "unstable"` (`#[cfg(any(test, all(desktop, feature = "unstable")))]` in `tauri-2.10.3/src/window/mod.rs`).

**Two spike paths, run in parallel, with split acceptance criteria:**

#### Path P0-A: `add_child` (full-fix candidate)

- **Step 0.A.1** Edit `src-tauri/Cargo.toml`: pin `tauri = { version = "=2.10.3", features = ["unstable", ...existing features] }`. Add inline SAFETY comment: `# SAFETY: tauri::Window::add_child is gated on the "unstable" feature; pinned to =2.10.3 because the API may change in any 2.x minor without semver protection.`
- **Step 0.A.2** Add a temporary debug command `spawn_test_embedded_webview(url)` in `src-tauri/src/webview.rs`:
  ```rust
  let main = app.get_webview_window("main")
      .ok_or_else(|| "main window not yet initialized".to_string())?;
  main.add_child(
      WebviewBuilder::new(label, WebviewUrl::External(parsed_url)),
      position, size,
  ).map_err(|e| e.to_string())?;
  ```
  No `.unwrap()` anywhere in the spike. Errors propagate as `Result<String, String>` to the frontend.
- **Step 0.A.3** Wire a temporary debug button to invoke it on `https://github.com` (a site that defeats iframes).
- **Step 0.A.4** Run the full Phase 5 manual matrix on all three platforms: drift on rapid resize, z-order vs. main app menus and dialogs, virtual-desktop / Spaces migration, devtools open/close without orphaning, 50x dispose/recreate.
- **Acceptance (P0-A — full fix):** All four defect classes resolved on Win11, macOS 14, Ubuntu 24.04. Screenshot/video evidence attached.

#### Path P0-B: Stable-API path (`WebviewWindowBuilder::parent()` + `tauri://move`)

- **Step 0.B.1** Build a sibling top-level `WebviewWindow` parented at construction time via `WebviewWindowBuilder::parent(&main)` (Windows-only owner relationship per Tauri 2.10 docs; on macOS/Linux the builder accepts the call but the underlying parent semantics differ). Note explicitly: this is *not* an embedded child — it remains a top-level window, just with an owner relationship on Windows.
- **Step 0.B.2** Replace 33ms polling with subscription to the main window's `tauri://move` and `tauri://resize` events; on each event recompute pane bounds and call `set_position` / `set_size` on the child.
- **Step 0.B.3** Run the same manual matrix.
- **Acceptance (P0-B — MVF only):** Drift defect resolved on all three platforms. Z-order, virtual-desktop migration, and devtools parity are *explicitly excluded* from the acceptance criteria — they are known structurally not to be fixed by this path.

#### Decision tree at the end of Phase 0

| Outcome | Decision |
| --- | --- |
| P0-A passes on all three platforms | Proceed with full Option A (Phase 1-6 as written). |
| P0-A passes on Windows + macOS, fails on Linux only | Use Option B (platform-direct, `gtk_container_add` / `webkit2gtk` socket) for Linux + Option A for Win/macOS. **Do NOT** fall back to P0-B for any platform — P0-B does not fix z-order/virtual-desktop. |
| P0-A fails on Windows | **Halt entirely.** Windows is the primary target per `Cargo.toml` manifest and is the platform with the largest current user share. Reconvene consensus. |
| P0-A fails on macOS only | Reconvene consensus to weigh Option B (macOS `addSubview`) vs. shipping Win+Linux first. |
| Only drift matters (user confirms scope reduction) and P0-B passes | Ship MVF (P0-B), update ADR with reduced-scope decision, close. Skip Phases 1-6. |
| P0-A and P0-B both fail | Escalate to Option C (CEF) reserve. Reconvene consensus. |

**Q5 resolution (pre-decided, no further user input required):** Linux-only failure -> Option B Linux + Option A Win/macOS. Windows failure -> halt. macOS-only failure -> reconvene.

### Phase 1 - Backend IPC surface (2 days)

- **Step 1.1** In `src-tauri/src/webview.rs`, add four commands paralleling the existing four but built on `WebviewBuilder`: `embedded_create`, `embedded_destroy`, `embedded_navigate`, `embedded_set_bounds`. Keep the old `create_webview` family untouched.
- **Step 1.2** Maintain a `Mutex<HashMap<String, Webview<R>>>` in app state keyed by pane `Uuid` so `set_bounds` and `navigate` can resolve the handle without re-walking the manager.
- **Step 1.3** Register the commands in `src-tauri/src/lib.rs`'s `invoke_handler`.
- **Step 1.4** Unit tests under `src-tauri/src/webview.rs` `#[cfg(test)]` for label collision and idempotent destroy. (The full integration path needs a desktop runtime and is covered in Phase 5.)
- **Acceptance:** `cargo build --features desktop` succeeds on all three platforms in CI. New commands are reachable from `pnpm tauri dev`.

### Phase 2 - Frontend bridge + pane class (2 days)

- **Step 2.1** Add four typed wrappers in `src/ipc/bridge.ts` next to the existing browser commands: `embeddedCreate`, `embeddedDestroy`, `embeddedNavigate`, `embeddedSetBounds`.
- **Step 2.2** Add `"embedded_browser"` to the `PaneKind` union in `src/types.ts`.
- **Step 2.3** Create `src/browser/EmbeddedBrowserPane.ts` implementing `Pane`. Pattern after `NativeBrowserPane.ts` but:
  - `element` is a placeholder `<div>` reserved by the layout engine; the actual webview renders on top of it via the Tauri child-webview API and is positioned to match `element.getBoundingClientRect()` whenever the layout marks the pane dirty.
  - `scheduleFit` -> single `embeddedSetBounds` call (no 33ms polling). Hook into the same layout dirty signal terminal panes use, plus a `ResizeObserver` on `element` as a safety net.
  - `dispose` -> `embeddedDestroy` then DOM removal.
- **Step 2.4** Extend `WorkspaceManager.createPane` (line 222 area) with an `if (spec.pane_kind === "embedded_browser")` branch returning the new class.
- **Acceptance:** Unit tests for `EmbeddedBrowserPane` lifecycle (mocked bridge). `pnpm test` green.

### Phase 3 - Layout integration + focus + URL bar (2 days)

- **Step 3.1** Reuse the URL-bar UI pattern from `BrowserPane` (port the relevant input/keyboard wiring into `EmbeddedBrowserPane` so the user sees the same affordance).
- **Step 3.2** Wire focus: when the pane gains focus in `WorkspaceManager`, call the new `embeddedFocus` command; when it loses focus, no-op (the platform compositor handles z-order). Verify keyboard shortcut handlers (line 112 area: "Don't steal focus from text inputs inside panes") still work because the placeholder div remains the focus owner.
- **Step 3.3** Persistence: extend `WorkspaceManager` snapshot/restore (lines 610, 621 area) to round-trip `pane_kind: "embedded_browser"` and the last URL.
- **Acceptance:** Split a pane into embedded browser, navigate, refocus terminal, refocus browser, resize parent window, drag splitter - no visible drift, no flicker, focus follows clicks.

### Phase 4 - Feature flag + opt-in path (1 day)

- **Step 4.1** Gate `embedded_browser` behind a runtime config in user settings (`features.embeddedBrowser: boolean`, default `false`). When off, the existing `browser` and `native_browser` paths are the only ones offered in UI.
- **Step 4.2** Add a build-time check that fails CI if both feature flags ship as default-on simultaneously.
- **Step 4.3** Update the "split into browser" action (line 384 area: `pane_kind: "browser"`) to choose `"embedded_browser"` when the feature flag is on.
- **Acceptance:** Off by default. Toggling on swaps in the new pane without restart for new panes; existing panes remain on the old class.

### Phase 5 - Cross-platform CI + manual matrix (1 day)

- **Step 5.1** Extend the existing CI workflow to add a smoke test that boots the app headlessly (or with xvfb on Linux), opens an embedded browser pane on `about:blank`, asserts no panic, and exits cleanly.
- **Step 5.2** Manual test matrix: documented in `.omc/plans/browser-engine-test-matrix.md`. Required platforms: Win11 (WebView2 Evergreen), macOS 14 (WKWebView), Ubuntu 24.04 (webkit2gtk). Required scenarios: resize, splitter drag, full-screen toggle, devtools, navigation, dispose-and-recreate 50x.
- **Acceptance:** All three platforms pass the manual matrix; CI green.

### Phase 6 - Docs + ADR + release notes (0.5 day)

- **Step 6.1** Write the ADR at `.omc/plans/adrs/browser-engine.md` (template in section 9).
- **Step 6.2** Update `README.md`, `README.ko.md`, `README.ja.md` "Features" with one bullet on the embedded browser flag, marked experimental.
- **Step 6.3** Add to `CHANGELOG`/release notes for the next version (post v0.8.4).
- **Acceptance:** Docs reviewed; ADR committed.

---

## 6. File Change Map

| Path | Change |
| --- | --- |
| `src-tauri/src/webview.rs` | Add `embedded_*` commands; keep existing intact |
| `src-tauri/src/lib.rs` | Register new commands in `invoke_handler` |
| `src-tauri/Cargo.toml` | Enable `features = ["unstable"]` on the existing `tauri` dep; pin to exact patch version `tauri = "=2.10.3"`; add SAFETY comment explaining the semver exemption (the `unstable` feature has no compatibility guarantee within 2.x). No new crates. (Option B/C would add new crates here.) |
| `src/types.ts` | Add `"embedded_browser"` to `PaneKind` |
| `src/ipc/bridge.ts` | Four new wrappers |
| `src/browser/EmbeddedBrowserPane.ts` | New file, ~250 LOC patterned on `NativeBrowserPane.ts` minus the polling |
| `src/workspace/WorkspaceManager.ts` | One extra branch in `createPane`; persistence round-trip; flag-gated split target |
| `src/config/*` (or equivalent) | New `features.embeddedBrowser` boolean |
| `.omc/plans/adrs/browser-engine.md` | ADR (new) |
| `.omc/plans/browser-engine-test-matrix.md` | Manual QA matrix (new) |
| `README.md`, `README.ko.md`, `README.ja.md` | Feature bullet |

No changes to: `TerminalPane.ts`, `Pane.ts` interface, layout engine internals, `yipc` crate.

---

## 7. Pre-mortem (DELIBERATE mode required)

Three concrete failure scenarios and the leading indicator for each.

### Scenario 1 - Tauri `add_child` has a fatal limitation we didn't anticipate

E.g. on macOS the child webview cannot be programmatically focused, or on Linux webkit2gtk does not honor `set_position` until the parent emits a configure event, or rapid create/destroy churn leaks GTK widgets.

- **Leading indicator:** Phase 0 spike fails on one platform, OR Phase 5 50x dispose-and-recreate test leaks memory or panics.
- **Mitigation:** Phase 0 is explicitly a gate. If it fails, the plan halts and consensus reconvenes to elevate Option B (platform-direct) or Option C (CEF) to the lead.
- **Recovery cost:** ~1 day spike work lost; no shipped code to revert.

### Scenario 1b - Tauri 2.x minor renames or restructures `add_child` (unstable API gate)

`Window::add_child` is gated on `feature = "unstable"` in tauri 2.10.3. The `unstable` feature explicitly carries no semver guarantee within the 2.x line — any minor (2.11, 2.12, ...) may rename, restructure, or remove the API.

- **Leading indicator:** `cargo update` on CI fails the build with `error[E0599]: no method named add_child found for struct Window` or similar. CI workflow includes a periodic `cargo update --dry-run` job (or dependabot/renovate explicit-upgrade-PR mode) that surfaces this before it lands on `main`.
- **Mitigation:** Pin `tauri = "=2.10.3"` exactly (not `^2.10.3`, not `~2.10.3`) in `src-tauri/Cargo.toml`. Configure `renovate.json` / `dependabot.yml` to require explicit PRs for any `tauri` bump, never auto-merge. SAFETY comment on the dep line documents the rationale for the exact pin so future maintainers don't relax it.
- **Recovery cost:** 1-3 days to adapt to the renamed/restructured API on the next deliberate Tauri bump, OR fall back to Option B (platform-direct) if the API is removed entirely without a stable replacement. Existing pinned releases continue to build indefinitely.

### Scenario 2 - Focus and IME interaction breaks for terminal panes when an embedded browser pane is present

Tauri/wry runs each child webview as its own input target on some platforms; keyboard events meant for the terminal pane's xterm.js could be captured by an adjacent browser webview, or IME composition windows could attach to the wrong webview.

- **Leading indicator:** During Phase 3 manual testing, typing in a terminal pane while a browser pane is in the same window drops keystrokes or routes them to the browser; or Korean/Japanese/Chinese IME breaks in either pane.
- **Mitigation:** Hook `embeddedSetFocus(false)` aggressively whenever the focused pane is not the embedded browser. Add an integration test that types into a terminal pane while a browser pane is mounted and verifies the terminal sees every keystroke. Document IME caveats in the ADR.
- **Recovery cost:** Up to 2 extra days in Phase 3 if mitigation requires a focus arbiter layer.

### Scenario 3 - Binary size or platform install surface grows despite "no new engine"

Even though Option A reuses the engine, the multi-webview path may pull in additional Tauri features or wry feature flags that were previously off, or the new commands may force the `desktop` feature ON for paths that used to compile lean.

- **Leading indicator:** Release-profile binary grows >5MB between v0.8.4 and the v0.9.x build; or `cargo build --no-default-features --lib --tests` (the lean cross-platform test path documented in `Cargo.toml:38`) starts requiring `desktop`.
- **Baseline capture (do this BEFORE Phase 1 starts):** Build v0.8.4 release-stripped binary on each of Win11, macOS 14, Ubuntu 24.04 using the project's existing release profile (`opt-level = "z"`, `lto = true`, `strip = true`). Record `cargo bloat --release -n 50` output and the final binary size (in bytes) per platform into `.omc/plans/baseline-binary-size-v0.8.4.md`.
- **Mitigation:** After Phase 1 lands (and again after Phase 4), re-run `cargo bloat --release -n 50` with the same command on each platform. Merge gate is **Δ < 1MB** vs. the v0.8.4 baseline on every platform. Keep `embedded_*` commands inside the existing `desktop` feature gate.
- **Recovery cost:** Refactor command gating; ~0.5 days.

---

## 8. Expanded Test Plan (DELIBERATE mode required)

### Unit (Rust)

- `webview.rs` label collision: creating two embedded webviews with the same pane id returns a typed error, not a panic.
- `webview.rs` destroy idempotency: destroying an already-destroyed label returns `Ok(())`.
- App-state map cleanup: on destroy, the `HashMap` entry is removed.

### Unit (TypeScript)

- `EmbeddedBrowserPane` lifecycle with mocked `bridge`: `spawn` calls `embeddedCreate` exactly once even when invoked twice (idempotency contract from `Pane.spawn`).
- `scheduleFit` debounces multiple synchronous calls into one `embeddedSetBounds`.
- `dispose` removes the DOM placeholder and calls `embeddedDestroy`.

### Integration

- Snapshot/restore round trip: persist a layout containing one terminal and one embedded browser pane, restart, verify both rehydrate with correct URL and bounds.
- Split, swap, dispose: open embedded browser, split it, swap orientations, close it - no orphaned webviews (assert via `embeddedList` debug command count).

### End-to-end (manual matrix on Win/macOS/Linux)

- Navigate to a site that blocks iframes (github.com) - succeeds.
- Resize the main window 30 times rapidly - no drift, no flicker.
- Drag the splitter between a terminal and the embedded browser - bounds track in real time.
- Toggle full-screen - browser stays parented and bounded.
- Open native devtools on the browser pane (Ctrl/Cmd+Shift+I) - works without orphaning the devtools window when the pane is closed.
- Switch virtual desktops / Spaces while a browser pane is open - browser follows the app, does not get left behind (this is the headline failure mode of the current `NativeBrowserPane`).

### Observability

- Add a `tracing::info!(label, kind="embedded", action)` call on each create/destroy/navigate/set_bounds in `webview.rs` so production logs let us reconstruct any reported drift incident.
- Frontend: `console.warn` if `scheduleFit` fires more than 10x/sec for a single pane (would indicate a feedback loop).
- Add a `bridge.metrics` counter (already present pattern in `bridge.ts`) for `embeddedSetBounds` calls per session to validate "no polling" empirically post-release.

### Performance / regression

- Cold-start with one terminal pane: no measurable delta (the new code path is dormant).
- Cold-start with one embedded browser pane vs. one `native_browser` pane: embedded should be at least as fast (no separate window creation cost). Acceptance: <=1.0x of `native_browser` time.
- 50x rapid create/destroy of an embedded browser pane: no leak >5MB resident, no handle/widget leak.

---

## 9. ADR Template (to be filled after consensus picks a path)

To be saved at `.omc/plans/adrs/browser-engine.md`:

- **Decision:** [chosen option]
- **Decision Drivers:** [from RALPLAN-DR section 11]
- **Alternatives considered:** A, B, C, plus invalidated D, E, F
- **Why chosen:** [from consensus rationale]
- **Consequences:**
  - Binary size delta: [measured Δ vs v0.8.4 baseline per platform]
  - License obligations: [none new for A; redistribution terms for C]
  - Maintenance load: [Tauri-shared for A; yMux-owned for B; dual-engine for C]
  - Platform parity status: [from Phase 0 + Phase 5 results]
  - **Unstable-feature dependency (Option A only):** This decision relies on the `tauri` crate `unstable` Cargo feature, which has no semver guarantee within the 2.x line. The `tauri` dep is pinned to exactly `=2.10.3`; any minor bump (2.11+) requires deliberate validation and may need `add_child` adaptation work. Renovate/Dependabot configured for explicit-upgrade PRs only, never auto-merge for `tauri`. If the API is removed without a stable equivalent, fall-through is to Option B (platform-direct) per Phase 0 decision tree.
- **Follow-ups:** [removal of legacy paths in vN+1, planned investigations, Tauri stable-API migration when `add_child` graduates from `unstable`]

---

## 10. Open Questions

(Will be persisted to `.omc/plans/open-questions.md` in the open-questions block at the end.)

- Q1. Is there a hard product reason to keep the iframe `BrowserPane.ts` after the embedded variant ships, or is iframe purely a transitional path?
- Q2. Should the URL bar live in the layout (Tauri-side, part of the pane chrome) or inside the embedded webview as a custom protocol-served page? (Affects DPI, theming, and z-order concerns differently.)
- Q3. Devtools UX: in-pane (right-click -> Inspect) or external window? (Tauri default is external; user may want in-pane.)
- Q4. Per-pane vs. shared web data: do we want each pane to use a fresh ephemeral storage partition (private-mode-by-default) or share cookies and login state across panes?
- ~~Q5. What if Phase 0 fails on one platform but not others?~~ **Resolved in iteration 2:** Linux-only failure -> Option B Linux + Option A Win/macOS. Windows failure -> halt (primary platform). macOS-only failure -> reconvene. (See Phase 0 decision tree.)
- Q6 (NEW). Scope confirmation: Is the work scoped to fix drift only (MVF stable-path is sufficient, 1-2 days), or all four defect classes — drift + z-order + virtual-desktop + devtools (full Option A, 8-9 days)? Default assumption: full scope. User confirmation requested before Phase 0 P0-B is even attempted, since the MVF only matters if scope is drift-only.

---

## 11. RALPLAN-DR Summary

### Mode

**DELIBERATE.** Cross-platform native engine surface, licensing exposure on alternatives, binary-size sensitivity (project is already careful with `default-features = false` and `opt-level = "z"`). High-risk signal triggers DELIBERATE per planner constraints.

### Principles (5)

1. **Reuse the engine you already ship.** The user has wry/WebView2/WKWebView/webkit2gtk in the binary. New engines must clear a high bar.
2. **Match terminal-pane mechanics.** Browser panes should feel like terminal panes: parented, no drift, focus arbitrated by the same path.
3. **Cross-platform parity is a release-blocker.** No Windows-only intermediate. Every change ships on all three platforms or doesn't ship.
4. **Reversibility.** New path lands behind a flag, side-by-side, for at least one release before the legacy paths are removed.
5. **Browser is secondary to terminal.** This work has a cost ceiling. We do not redesign architecture to make a browser pane perfect; we do the smallest change that fixes the polling/iframe problems.

### Decision Drivers (top 3)

1. **Binary size and license footprint.** Project ships lean (`opt-level = "z"`, `lto = true`, `panic = "abort"`, `rustls-tls`, `strip = true`). A 100-150MB CEF blob or a commercial Ultralight license is a major regression of stated project values.
2. **Maintenance load.** yMux is a small project; every native dependency is a long-tail cost (CVEs, bundling, platform installer changes). Reusing the engine Tauri already maintains is multiplicatively cheaper.
3. **Embedding correctness on all three OSes without per-platform code paths.** The current pain point is geometry/parenting, which is squarely an embedding-model issue. Whichever option fixes embedding correctness with the least platform-specific code wins.

### Viable Options

#### Option A (RECOMMENDED) - Tauri 2 multi-webview (`Window::add_child` + `WebviewBuilder`)

Use the multi-webview-per-window API confirmed present in `tauri-2.10.3/src/window/mod.rs:1052`. Same engine, cross-platform automatically.

- **Pros:**
  - Zero binary delta (engine already shipped).
  - Cross-platform automatic via wry.
  - Parent/child relationship native; no polling; correct z-order.
  - API surface mirrors today's `webview.rs` 1:1; small code delta.
  - Tauri team owns the maintenance burden of the multi-webview API.
- **Cons:**
  - `add_child` is younger than `WebviewWindow` and may have rough edges (devtools, transparency, rapid churn) on at least one platform.
  - Dependent on Tauri's release cadence for any embedding bugs we hit.
  - No process isolation between webviews on the same window (shared crash surface; one runaway page can stutter the host).

#### Option B - Platform-direct embedding (`webview2-com` + `SetParent` on Windows, `addSubview` / `WKWebView` on macOS, `gtk_container_add` on Linux)

Skip Tauri's child-webview API and parent webviews directly via OS calls.

- **Pros:**
  - Finest-grained control over geometry, z-order, focus, and process model.
  - Not blocked by Tauri's roadmap if `add_child` regresses.
- **Cons:**
  - Triple the code: separate implementation per platform with platform-specific bug surface.
  - Crosses Tauri's abstraction boundary; future Tauri upgrades may break our assumptions.
  - Substantially more `unsafe` and FFI surface to maintain.
  - Likely 3x the schedule.

#### Option C - CEF (Chromium Embedded Framework) via `cef` crate

Drop wry for browser panes and embed full CEF.

- **Pros:**
  - Full Chromium feature parity: DRM media (Widevine), advanced web platform features, identical devtools.
  - Process isolation per webview (one-tab-crash-doesn't-kill-host model).
  - Mature embedding API.
- **Cons:**
  - **~150MB binary growth** (CEF redistributable). Direct conflict with Decision Driver 1.
  - Adds a second engine to maintain alongside wry (wry remains for the main window).
  - License is permissive but the redistributable bundling adds installer complexity per platform.
  - Not necessary to fix the stated problems (iframe restrictions and polling) - both are fully solved by Option A.

### Invalidated Alternatives (explicit rationale)

- **Ultralight.** Commercial license risk for an open-source project (free tier has annual revenue caps and project-count limits). Missing WebGL2/some modern web platform features. Engine quality not on par with Chromium for general-purpose browsing. Invalidated.
- **Servo (embedded).** Embedding API is not production-ready; project velocity is uncertain post-Mozilla; would block on upstream features for general-site compatibility. Invalidated.
- **"Fix the current `NativeBrowserPane` overlay via Win32 `SetParent` only."** Violates Principle 3 (cross-platform parity). Even if it shipped, the macOS and Linux equivalents would still need to be written, at which point Option A or B is strictly superior. Invalidated.
- **Headless Chromium + screenshot streaming.** No keyboard interactivity for forms, video, or developer workflows; defeats the purpose of a browser pane. Invalidated.

### Comparative Summary

| Driver | A (multi-webview) | B (platform-direct) | C (CEF) |
| --- | --- | --- | --- |
| Binary size delta | ~0 MB | ~0 MB | ~150 MB |
| License surface | none new | none new | redistribution terms |
| Cross-platform code | one path *(API surface is one path; underlying wry implementations differ per platform — Phase 0 validates parity)* | three paths | one path (CEF abstracts) |
| Schedule estimate | 8-9 days | 18-22 days | 20-25 days + bundling |
| Maintenance load | Tauri-shared | yMux-owned | dual-engine |
| Recommendation | **Lead** | Fallback if A's spike fails | Only if Chromium-specific features (DRM, etc.) become a product requirement |

### Recommendation

Lead with **Option A** behind the `tauri` `unstable` feature flag, with the dep pinned to `=2.10.3`. Phase 0 runs P0-A (full fix candidate) and P0-B (MVF stable-path candidate) **in parallel** with split acceptance criteria — see Phase 0 decision tree. Failure modes are pre-decided:

- Linux-only failure on P0-A -> Option B for Linux + Option A for Windows/macOS.
- Windows failure on P0-A -> halt entirely (Windows is the primary platform per `Cargo.toml` manifest target).
- macOS-only failure -> reconvene consensus.
- User-confirmed scope reduction to drift-only -> ship MVF (P0-B) and close.

Option C (CEF) is held in reserve for a future product-driven decision (DRM video, full Chromium devtools parity), not this work.

---

## 12. Confirmation

**Plan saved to:** `.omc/plans/browser-engine-upgrade.md`

**Scope:**
- Two committed branches gated by Phase 0 spike:
  - **MVF branch** (drift-only fix): 1-2 engineering days, P0-B stable-path only.
  - **Full Option A**: 6 phases, ~8.5-10 engineering days (Phase 0 widened to 1-2 days for parallel spike).
- ~3 backend files, ~4 frontend files, 2 doc files (full path).

**Estimated complexity:** MEDIUM (well-scoped surface, but cross-platform native code with native focus/IME interactions in a non-trivial layout engine, and reliance on the `unstable` Tauri Cargo feature which has no semver guarantee within 2.x).

**Key Deliverables:**
1. New `embedded_browser` pane variant behind a feature flag
2. Backend `embedded_*` Tauri commands using `WebviewBuilder` + `Window::add_child`
3. Frontend `EmbeddedBrowserPane` class with ResizeObserver-driven (not polled) bounds sync
4. Cross-platform CI + manual test matrix
5. ADR documenting the decision and its consequences

**Consensus mode:**
- RALPLAN-DR: 5 principles, 3 drivers, 3 viable options + 4 invalidated with rationale
- ADR template ready; final fields filled after consensus selects a path

**Does this plan capture your intent?**
- "proceed" - Begin implementation via /oh-my-claudecode:start-work
- "adjust [X]" - Return to interview to modify
- "restart" - Discard and start fresh

# yDir: Open Non-Binary Files in yCode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the user presses Enter on a non-binary file in yDir, suspend the yDir TUI and open the file in yCode; resume yDir when yCode exits.

**Architecture:** Add an `open_in_ycode: Option<PathBuf>` field to `App`; `enter_dir()` sets it when a non-binary, non-executable file is selected. In `main.rs`'s event loop, after `enter_dir()` returns, check that field — if set, suspend the TUI, spawn `ycode <path>`, wait for exit, then resume. Binary detection reads the first 8192 bytes and checks for null bytes (same heuristic as git).

**Tech Stack:** Rust, ratatui, crossterm — same stack already used by yDir.

---

## File Map

| File | Change |
|------|--------|
| `tools/ydir/src/app.rs` | Add `open_in_ycode` field, `is_binary_file()` fn, update `enter_dir()` and `App::new()` |
| `tools/ydir/src/main.rs` | Handle `open_in_ycode` signal: suspend → run ycode → resume |

`tools/ydir/src/ui.rs` requires **no changes** — the footer already reads `Enter Open`, which stays correct.

---

### Task 1: Add `is_binary_file()` and wire up `App`

**Files:**
- Modify: `tools/ydir/src/app.rs`

- [ ] **Step 1: Write the failing test**

Add inside the existing `#[cfg(test)]` block at the bottom of `tools/ydir/src/app.rs`:

```rust
#[test]
fn binary_detection_null_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("data.bin");
    fs::write(&bin, b"some\x00binary\x00data").unwrap();
    assert!(super::is_binary_file(&bin), "should detect null bytes as binary");
}

#[test]
fn binary_detection_text_file() {
    let tmp = tempfile::tempdir().unwrap();
    let txt = tmp.path().join("readme.md");
    fs::write(&txt, b"# Hello\nThis is plain text.\n").unwrap();
    assert!(!super::is_binary_file(&txt), "plain text should not be binary");
}

#[test]
fn binary_detection_missing_file() {
    let path = std::path::Path::new("/nonexistent/file.bin");
    // Missing files should not be treated as binary (fail safe → open attempt)
    assert!(!super::is_binary_file(path));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cargo test -p ydir -- binary_detection
```

Expected: compile error — `is_binary_file` not defined yet.

- [ ] **Step 3: Add `is_binary_file()` function**

Add this function just above the existing `is_executable()` function in `tools/ydir/src/app.rs`:

```rust
pub fn is_binary_file(path: &std::path::Path) -> bool {
    use std::io::Read;
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let mut buf = [0u8; 8192];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return false,
    };
    buf[..n].contains(&0u8)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cargo test -p ydir -- binary_detection
```

Expected: 3 tests pass.

- [ ] **Step 5: Add `open_in_ycode` field to `App` and update `enter_dir()`**

In `tools/ydir/src/app.rs`, update the `App` struct definition:

```rust
pub struct App {
    pub left: Panel,
    pub right: Panel,
    pub active: PanelSide,
    pub show_hidden: bool,
    pub clipboard: Option<(PathBuf, ClipboardOp)>,
    pub status_msg: Option<String>,
    pub run_dialog: Option<RunDialog>,
    pub open_in_ycode: Option<PathBuf>,  // ← add this line
}
```

Update `App::new()` to initialize it:

```rust
pub fn new(start_dir: PathBuf) -> Result<Self> {
    let left = Panel::new(&start_dir)?;
    let right = Panel::new(&start_dir)?;
    Ok(Self {
        left,
        right,
        active: PanelSide::Left,
        show_hidden: false,
        clipboard: None,
        status_msg: None,
        run_dialog: None,
        open_in_ycode: None,  // ← add this line
    })
}
```

Update `enter_dir()` — replace the final `else` branch (currently `"Not executable: ..."`) with binary-aware logic:

```rust
pub fn enter_dir(&mut self) -> Result<()> {
    let entry = self.active_panel().selected_entry().cloned();
    if let Some(entry) = entry {
        if entry.is_dir {
            let show_hidden = self.show_hidden;
            let panel = self.active_panel_mut();
            panel.cwd = entry.path;
            panel.selected = 0;
            panel.reload(show_hidden)?;
        } else if is_executable(&entry.name) {
            self.run_dialog = Some(RunDialog {
                file_name: entry.name.clone(),
                file_path: entry.path.clone(),
                args_input: String::new(),
            });
        } else if is_binary_file(&entry.path) {
            self.status_msg = Some(format!("Binary file: {}", entry.name));
        } else {
            self.open_in_ycode = Some(entry.path.clone());
        }
    }
    Ok(())
}
```

- [ ] **Step 6: Write a test for the new `enter_dir()` behaviour**

Add inside the existing `#[cfg(test)]` block:

```rust
#[test]
fn enter_on_text_file_sets_open_in_ycode() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("notes.txt"), "hello world").unwrap();
    let mut app = App::new(tmp.path().to_path_buf()).unwrap();
    let idx = app.left.entries.iter().position(|e| e.name == "notes.txt").unwrap();
    app.left.selected = idx;
    app.enter_dir().unwrap();
    assert!(app.open_in_ycode.is_some());
    assert!(app.run_dialog.is_none());
    assert!(app.status_msg.is_none());
}

#[test]
fn enter_on_binary_file_shows_status() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("data.bin"), b"bin\x00data").unwrap();
    let mut app = App::new(tmp.path().to_path_buf()).unwrap();
    let idx = app.left.entries.iter().position(|e| e.name == "data.bin").unwrap();
    app.left.selected = idx;
    app.enter_dir().unwrap();
    assert!(app.open_in_ycode.is_none());
    assert!(app.status_msg.as_ref().unwrap().contains("Binary file"));
}
```

- [ ] **Step 7: Run full ydir test suite**

```
cargo test -p ydir
```

Expected: all tests pass (including the existing `enter_on_non_executable_shows_status` test — note: that test uses `readme.md` which is plain text, so it will now fail because we changed the behaviour). Update that test:

```rust
#[test]
fn enter_on_non_executable_shows_status() {
    // This test now checks binary files, not plain text files
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("data.bin"), b"bin\x00data").unwrap();
    let mut app = App::new(tmp.path().to_path_buf()).unwrap();
    let idx = app
        .left
        .entries
        .iter()
        .position(|e| e.name == "data.bin")
        .unwrap();
    app.left.selected = idx;
    app.enter_dir().unwrap();
    assert!(app.run_dialog.is_none());
    assert!(app.status_msg.as_ref().unwrap().contains("Binary file"));
}
```

Run again:

```
cargo test -p ydir
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```
git add tools/ydir/src/app.rs
git commit -m "feat(ydir): add is_binary_file() and open_in_ycode signal"
```

---

### Task 2: Handle `open_in_ycode` in the event loop

**Files:**
- Modify: `tools/ydir/src/main.rs`

- [ ] **Step 1: Update the Enter key handler in `main.rs`**

In `tools/ydir/src/main.rs`, locate the `KeyCode::Enter => app.enter_dir()?,` line (currently inside the `else` branch at the bottom of the key handler). Replace it with:

```rust
KeyCode::Enter => {
    app.enter_dir()?;
    if let Some(path) = app.open_in_ycode.take() {
        disable_raw_mode()?;
        terminal.backend_mut().execute(LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        let status = std::process::Command::new("ycode")
            .arg(&path)
            .status();
        if let Err(e) = status {
            eprintln!("\nFailed to launch ycode: {}", e);
            eprintln!("Press Enter to return to yDir...");
            let _ = std::io::stdin().read_line(&mut String::new());
        }

        enable_raw_mode()?;
        terminal.backend_mut().execute(EnterAlternateScreen)?;
        terminal.clear()?;
        let _ = app.refresh();
    }
}
```

This suspend/resume pattern exactly mirrors what the RunDialog path already does in the same file. No new imports needed — `disable_raw_mode`, `enable_raw_mode`, `LeaveAlternateScreen`, `EnterAlternateScreen` are all already imported at the top.

The error path (ycode not in PATH) shows a message and waits for Enter so the user can read it. The success path (ycode exits normally) returns immediately to yDir.

- [ ] **Step 2: Verify it compiles**

```
cargo check -p ydir
```

Expected: no errors.

- [ ] **Step 3: Run full test suite one more time**

```
cargo test -p ydir
```

Expected: all tests pass (main.rs changes don't affect unit tests since they test app logic only).

- [ ] **Step 4: Commit**

```
git add tools/ydir/src/main.rs
git commit -m "feat(ydir): open non-binary files in ycode on Enter"
```

---

### Task 3: Verify end-to-end

- [ ] **Step 1: Build both tools**

```
cargo build -p ydir -p ycode
```

Expected: both compile successfully with no warnings.

- [ ] **Step 2: Manual smoke test**

Run ydir pointing at a directory that has both text and binary files:

```
cargo run -p ydir -- .
```

Test the following scenarios:

| Action | Expected result |
|--------|----------------|
| Navigate to a `.rs` / `.md` / `.toml` text file, press Enter | yDir suspends, yCode opens with the file loaded |
| Exit yCode (Esc → confirm) | yDir resumes at same location |
| Navigate to a binary (e.g. a `.exe` or compiled object), press Enter | Status bar shows `Binary file: <name>` |
| Navigate to an executable (`.exe`, `.bat`), press Enter | RunDialog appears (unchanged) |
| Navigate into a directory, press Enter | Directory is entered (unchanged) |

- [ ] **Step 3: Run full workspace test suite**

```
cargo test --no-default-features --lib -p ymux
cargo test -p ytheme -p yipc -p ymon -p ydir -p ycode -p ylauncher
```

Expected: all pass.

- [ ] **Step 4: Final commit (if any fixups needed)**

If step 3 uncovered issues, fix and commit. Otherwise this task is done.

use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Graph,
    Branches,
}

pub struct App {
    pub log_lines: Vec<String>,
    pub branches: Vec<String>,
    pub log_scroll: usize,
    pub branch_idx: usize,
    pub focus: Focus,
    pub status: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            log_lines: Vec::new(),
            branches: Vec::new(),
            log_scroll: 0,
            branch_idx: 0,
            focus: Focus::Graph,
            status: None,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.log_lines = run_git_log();
        self.branches = run_git_branches();
        if self.branch_idx >= self.branches.len() && !self.branches.is_empty() {
            self.branch_idx = self.branches.len() - 1;
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::Graph => Focus::Branches,
            Focus::Branches => Focus::Graph,
        };
    }

    pub fn scroll_up(&mut self) {
        match self.focus {
            Focus::Graph => {
                self.log_scroll = self.log_scroll.saturating_sub(1);
            }
            Focus::Branches => {
                self.branch_idx = self.branch_idx.saturating_sub(1);
            }
        }
    }

    pub fn scroll_down(&mut self) {
        match self.focus {
            Focus::Graph => {
                let max = self.log_lines.len().saturating_sub(1);
                if self.log_scroll < max {
                    self.log_scroll += 1;
                }
            }
            Focus::Branches => {
                let max = self.branches.len().saturating_sub(1);
                if self.branch_idx < max {
                    self.branch_idx += 1;
                }
            }
        }
    }

    pub fn checkout_selected(&mut self) {
        let Some(raw) = self.branches.get(self.branch_idx) else {
            return;
        };
        let branch = raw.trim().trim_start_matches("* ").to_string();

        match Command::new("git").args(["checkout", &branch]).output() {
            Ok(out) if out.status.success() => {
                self.status = Some(format!("Checked out: {branch}"));
                self.refresh();
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                self.status = Some(format!("Error: {}", err.trim()));
            }
            Err(e) => {
                self.status = Some(format!("Failed to run git: {e}"));
            }
        }
    }
}

fn run_git_log() -> Vec<String> {
    match Command::new("git")
        .args([
            "log",
            "--graph",
            "--oneline",
            "--all",
            "--decorate",
            "--color=never",
        ])
        .output()
    {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .collect(),
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            vec![format!("git error: {}", err.trim())]
        }
        Err(e) => vec![format!("failed to run git: {e}")],
    }
}

fn run_git_branches() -> Vec<String> {
    match Command::new("git").args(["branch"]).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_does_not_panic() {
        let _app = App::new();
    }

    #[test]
    fn refresh_does_not_panic() {
        let mut app = App::new();
        app.refresh();
    }

    #[test]
    fn focus_toggles() {
        let mut app = App::new();
        assert_eq!(app.focus, Focus::Graph);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Branches);
        app.toggle_focus();
        assert_eq!(app.focus, Focus::Graph);
    }

    #[test]
    fn log_scroll_up_at_zero_stays_zero() {
        let mut app = App::new();
        app.log_scroll = 0;
        app.scroll_up();
        assert_eq!(app.log_scroll, 0);
    }

    #[test]
    fn branch_scroll_up_at_zero_stays_zero() {
        let mut app = App::new();
        app.focus = Focus::Branches;
        app.branch_idx = 0;
        app.scroll_up();
        assert_eq!(app.branch_idx, 0);
    }

    #[test]
    fn log_scroll_down_bounded() {
        let mut app = App::new();
        let max = app.log_lines.len().saturating_sub(1);
        for _ in 0..max + 5 {
            app.scroll_down();
        }
        assert!(app.log_scroll <= max);
    }

    #[test]
    fn branch_scroll_down_bounded() {
        let mut app = App::new();
        app.focus = Focus::Branches;
        let max = app.branches.len().saturating_sub(1);
        for _ in 0..max + 5 {
            app.scroll_down();
        }
        assert!(app.branch_idx <= max);
    }

    #[test]
    fn checkout_noop_on_empty_branches() {
        let mut app = App::new();
        app.branches.clear();
        app.checkout_selected(); // must not panic
    }
}

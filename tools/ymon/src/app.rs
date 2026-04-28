use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Cpu,
    Memory,
    Processes,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Overview, Tab::Cpu, Tab::Memory, Tab::Processes];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Cpu => "CPU",
            Tab::Memory => "Memory",
            Tab::Processes => "Processes",
        }
    }
}

pub struct CpuSnapshot {
    pub name: String,
    pub usage: f32,
}

pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu: f32,
    pub memory_mb: f64,
}

pub struct App {
    pub sys: System,
    pub disks: Disks,
    pub networks: Networks,
    pub active_tab: usize,
    pub scroll_offset: usize,

    // Cached snapshots, updated each tick
    pub cpu_snapshots: Vec<CpuSnapshot>,
    pub processes: Vec<ProcessInfo>,
    pub cpu_history: Vec<f32>,
    pub mem_history: Vec<f32>,
}

const MAX_HISTORY: usize = 60;

impl App {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();

        let mut app = Self {
            sys,
            disks,
            networks,
            active_tab: 0,
            scroll_offset: 0,
            cpu_snapshots: Vec::new(),
            processes: Vec::new(),
            cpu_history: Vec::new(),
            mem_history: Vec::new(),
        };
        app.refresh_snapshots();
        app
    }

    pub fn tick(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh(true);
        self.networks.refresh(true);
        self.refresh_snapshots();
    }

    fn refresh_snapshots(&mut self) {
        self.cpu_snapshots = self
            .sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, cpu)| CpuSnapshot {
                name: format!("CPU {}", i),
                usage: cpu.cpu_usage(),
            })
            .collect();

        let global_cpu = self.sys.global_cpu_usage();
        self.cpu_history.push(global_cpu);
        if self.cpu_history.len() > MAX_HISTORY {
            self.cpu_history.remove(0);
        }

        let total_mem = self.sys.total_memory() as f64;
        let used_mem = self.sys.used_memory() as f64;
        let mem_pct = if total_mem > 0.0 {
            (used_mem / total_mem * 100.0) as f32
        } else {
            0.0
        };
        self.mem_history.push(mem_pct);
        if self.mem_history.len() > MAX_HISTORY {
            self.mem_history.remove(0);
        }

        let mut procs: Vec<ProcessInfo> = self
            .sys
            .processes()
            .values()
            .map(|p| ProcessInfo {
                pid: p.pid().as_u32(),
                name: p.name().to_string_lossy().into_owned(),
                cpu: p.cpu_usage(),
                memory_mb: p.memory() as f64 / 1024.0 / 1024.0,
            })
            .collect();
        procs.sort_by(|a, b| {
            b.cpu
                .partial_cmp(&a.cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        procs.truncate(200);
        self.processes = procs;
    }

    pub fn tab(&self) -> Tab {
        Tab::ALL[self.active_tab]
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % Tab::ALL.len();
        self.scroll_offset = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = if self.active_tab == 0 {
            Tab::ALL.len() - 1
        } else {
            self.active_tab - 1
        };
        self.scroll_offset = 0;
    }

    pub fn set_tab(&mut self, idx: usize) {
        if idx < Tab::ALL.len() {
            self.active_tab = idx;
            self.scroll_offset = 0;
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn total_mem_gb(&self) -> f64 {
        self.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn used_mem_gb(&self) -> f64 {
        self.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn total_swap_gb(&self) -> f64 {
        self.sys.total_swap() as f64 / 1024.0 / 1024.0 / 1024.0
    }

    pub fn used_swap_gb(&self) -> f64 {
        self.sys.used_swap() as f64 / 1024.0 / 1024.0 / 1024.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_initializes_without_panic() {
        let app = App::new();
        assert!(!app.cpu_snapshots.is_empty() || app.sys.cpus().is_empty());
    }

    #[test]
    fn tab_cycling() {
        let mut app = App::new();
        assert_eq!(app.tab(), Tab::Overview);
        app.next_tab();
        assert_eq!(app.tab(), Tab::Cpu);
        app.next_tab();
        assert_eq!(app.tab(), Tab::Memory);
        app.next_tab();
        assert_eq!(app.tab(), Tab::Processes);
        app.next_tab();
        assert_eq!(app.tab(), Tab::Overview);
        app.prev_tab();
        assert_eq!(app.tab(), Tab::Processes);
    }

    #[test]
    fn set_tab_bounds() {
        let mut app = App::new();
        app.set_tab(2);
        assert_eq!(app.tab(), Tab::Memory);
        app.set_tab(99);
        assert_eq!(app.tab(), Tab::Memory); // unchanged
    }

    #[test]
    fn tick_updates_history() {
        let mut app = App::new();
        let initial_len = app.cpu_history.len();
        app.tick();
        assert!(app.cpu_history.len() >= initial_len);
        assert!(app.mem_history.len() >= 1);
    }

    #[test]
    fn scroll_stays_in_bounds() {
        let mut app = App::new();
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_down();
        assert_eq!(app.scroll_offset, 1);
        app.scroll_up();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn memory_values_are_sensible() {
        let app = App::new();
        assert!(app.total_mem_gb() > 0.0);
        assert!(app.used_mem_gb() >= 0.0);
        assert!(app.used_mem_gb() <= app.total_mem_gb());
    }

    #[test]
    fn processes_sorted_by_cpu_desc() {
        let app = App::new();
        for w in app.processes.windows(2) {
            assert!(
                w[0].cpu >= w[1].cpu,
                "processes not sorted: {} ({}) vs {} ({})",
                w[0].name,
                w[0].cpu,
                w[1].name,
                w[1].cpu,
            );
        }
    }

    #[test]
    fn tab_labels() {
        assert_eq!(Tab::Overview.label(), "Overview");
        assert_eq!(Tab::Cpu.label(), "CPU");
        assert_eq!(Tab::Memory.label(), "Memory");
        assert_eq!(Tab::Processes.label(), "Processes");
    }
}

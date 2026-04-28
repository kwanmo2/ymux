use std::io;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

mod app;
mod ui;

use app::App;

fn main() -> Result<()> {
    let start_dir = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| ".".into()));

    let mut stdout = io::stdout();
    enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, start_dir);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    start_dir: std::path::PathBuf,
) -> Result<()> {
    let mut app = App::new(start_dir)?;

    loop {
        terminal.draw(|frame| ui::draw(frame, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Tab => app.toggle_panel(),
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Home => app.move_to_top(),
                        KeyCode::End => app.move_to_bottom(),
                        KeyCode::Enter => app.enter_dir()?,
                        KeyCode::Backspace => app.go_parent()?,
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.refresh()?;
                        }
                        KeyCode::Char('d') => app.delete_selected()?,
                        KeyCode::Char('c') => app.mark_copy(),
                        KeyCode::Char('m') => app.mark_move(),
                        KeyCode::Char('p') => app.paste()?,
                        KeyCode::Char('.') => app.toggle_hidden(),
                        _ => {}
                    }
                }
            }
        }
    }
}

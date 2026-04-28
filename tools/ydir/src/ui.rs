use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Panel, PanelSide};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    draw_panel(frame, &app.left, panels[0], app.active == PanelSide::Left);
    draw_panel(frame, &app.right, panels[1], app.active == PanelSide::Right);
    draw_footer(frame, app, chunks[1]);
}

fn draw_panel(frame: &mut Frame, panel: &Panel, area: Rect, active: bool) {
    let border_style = if active {
        Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))
    } else {
        Style::default().fg(Color::Rgb(0x1e, 0x2a, 0x38))
    };

    let title = format!(" {} ", panel.cwd.display());
    let block = Block::default()
        .title(title)
        .title_style(if active {
            Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca)).bold()
        } else {
            Style::default().fg(Color::Rgb(0x6a, 0x7a, 0x8a))
        })
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if panel.entries.is_empty() {
        let empty =
            Paragraph::new("(empty)").style(Style::default().fg(Color::Rgb(0x6a, 0x7a, 0x8a)));
        frame.render_widget(empty, inner);
        return;
    }

    let visible_height = inner.height as usize;
    let scroll = if panel.selected >= visible_height {
        panel.selected - visible_height + 1
    } else {
        0
    };

    let rows: Vec<Row> = panel
        .entries
        .iter()
        .skip(scroll)
        .enumerate()
        .map(|(i, entry)| {
            let idx = i + scroll;
            let style = if idx == panel.selected && active {
                Style::default()
                    .bg(Color::Rgb(0x1a, 0x22, 0x30))
                    .fg(Color::Rgb(0x7f, 0xdb, 0xca))
                    .add_modifier(Modifier::BOLD)
            } else if idx == panel.selected {
                Style::default()
                    .bg(Color::Rgb(0x1a, 0x22, 0x30))
                    .fg(Color::Rgb(0xd6, 0xde, 0xeb))
            } else if entry.is_dir {
                Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))
            } else {
                Style::default().fg(Color::Rgb(0xd6, 0xde, 0xeb))
            };

            let icon = if entry.is_dir { "📁" } else { "  " };
            let date = entry
                .modified
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            Row::new(vec![
                Cell::from(format!("{} {}", icon, entry.name)),
                Cell::from(entry.size_display()),
                Cell::from(date),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(50),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(vec!["Name", "Size", "Modified"])
            .style(Style::default().fg(Color::Rgb(0x6a, 0x7a, 0x8a)))
            .bottom_margin(0),
    );

    frame.render_widget(table, inner);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let status = app.status_msg.as_deref().unwrap_or("");
    let text = Line::from(vec![
        Span::styled("q", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Quit  "),
        Span::styled("Tab", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Switch  "),
        Span::styled("Enter", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Open  "),
        Span::styled("BS", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Parent  "),
        Span::styled(".", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Hidden  "),
        Span::styled("c/m/p/d", Style::default().fg(Color::Rgb(0x7f, 0xdb, 0xca))),
        Span::raw(" Copy/Move/Paste/Del  "),
        Span::styled(status, Style::default().fg(Color::Rgb(0xe5, 0xc0, 0x7b))),
    ]);
    frame.render_widget(Paragraph::new(text), area);
}

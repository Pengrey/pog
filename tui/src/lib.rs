mod app;
mod tabs;
pub mod widgets;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use models::{Asset, Finding, GraphData};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Tabs},
    Terminal,
};
use std::io::{self, stdout};

use app::App;

/// Launch the TUI with the provided data.
pub fn run_with_data(graph_data: GraphData, findings: Vec<Finding>, assets: Vec<Asset>) -> io::Result<()> {
    // Install a panic hook that restores the terminal before printing
    // the panic message. Without this, a panic leaves the terminal in
    // raw mode, making it unusable.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture);
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(graph_data, findings, assets);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(f.area());

            let titles: Vec<Line> = app.tab_titles().iter().map(|t| Line::from(*t)).collect();
            let tabs = Tabs::new(titles)
                .block(Block::default().borders(Borders::ALL).title(" pog (t: switch tab, q: quit) "))
                .select(app.current_tab_index())
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD));
            f.render_widget(tabs, chunks[0]);

            app.render_current_tab(f, chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    if !app.handle_key(key.code)
                        && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
                    {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    if let MouseEventKind::Down(_) = mouse.kind {
                        if mouse.row >= 1 && mouse.row <= 2 {
                            // Compute actual tab hit regions from title widths.
                            // ratatui Tabs renders:  " title1 │ title2 │ title3 "
                            // Border left edge takes 1 column.
                            let titles = app.tab_titles();
                            let mut x: usize = 1; // skip left border
                            let col = mouse.column as usize;
                            let mut clicked = None;
                            for (i, t) in titles.iter().enumerate() {
                                let w = t.len() + 2; // " title "
                                if col >= x && col < x + w {
                                    clicked = Some(i);
                                    break;
                                }
                                x += w + 1; // +1 for the "│" separator
                            }
                            if let Some(idx) = clicked {
                                app.select_tab(idx);
                            }
                        } else {
                            app.handle_click(mouse.column, mouse.row);
                        }
                    } else if let MouseEventKind::ScrollDown = mouse.kind {
                        app.handle_scroll_down();
                    } else if let MouseEventKind::ScrollUp = mouse.kind {
                        app.handle_scroll_up();
                    }
                }
                _ => {}
            }
        }
    }
}
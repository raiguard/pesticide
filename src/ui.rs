use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use std::thread;
use std::{io, time::Duration};
use tui::layout::{Constraint, Direction, Layout};
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

fn ui_demo() -> Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                ]
                .as_ref(),
            )
            .split(f.size());
        let block = Block::default().title("Variables").borders(Borders::ALL);
        f.render_widget(block, chunks[0]);
        let block = Block::default().title("Watch").borders(Borders::ALL);
        f.render_widget(block, chunks[1]);
        let block = Block::default().title("Call Stack").borders(Borders::ALL);
        f.render_widget(block, chunks[2]);
        let block = Block::default().title("Breakpoints").borders(Borders::ALL);
        f.render_widget(block, chunks[3]);
    })?;

    thread::sleep(Duration::from_millis(5000));

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

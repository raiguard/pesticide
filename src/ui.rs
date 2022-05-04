use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::widgets::{Block, Borders};
use tui::Terminal;

use crate::adapter::Adapter;

pub fn start(adapter: Arc<Mutex<Adapter>>) -> Result<()> {
    // HACK: XDG_DESKTOP_PORTALS is showing some sort of error that is throwing everything off
    thread::sleep(Duration::from_millis(300));

    enable_raw_mode()?;
    // Set up terminal
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Draw stuff!
    // TODO: Send a signal to this thread to update the UI when adapter state changes
    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Variables").borders(Borders::ALL);
        f.render_widget(block, size);
    })?;
    thread::sleep(Duration::from_millis(5000));

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

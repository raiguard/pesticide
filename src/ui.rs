use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::thread::{self, JoinHandle};
use tui::backend::CrosstermBackend;
use tui::widgets::{Block, Borders};
use tui::Terminal;

pub fn start() -> Result<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)> {
    // Handle input events on a separate thread to remove the need for a
    // timeout and allow use of the select! macro
    let (tx, rx) = crossbeam_channel::unbounded::<UiEvent>();
    let input = thread::spawn(move || -> Result<()> { input_thread(tx) });
    let ui = thread::spawn(move || -> Result<()> { ui_thread(rx) });

    Ok((input, ui))
}

fn input_thread(tx: Sender<UiEvent>) -> Result<()> {
    loop {
        match read()? {
            Event::Key(event) => match event.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                _ => (),
            },
            Event::Mouse(event) => debug!("{:?}", event),
            Event::Resize(width, height) => tx.send(UiEvent::Resize(width, height))?,
        }
    }

    Ok(())
}

fn ui_thread(rx: Receiver<UiEvent>) -> Result<()> {
    // Prepare terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Draw stuff!
    terminal.draw(|f| {
        let size = f.size();
        let block = Block::default().title("Variables").borders(Borders::ALL);
        f.render_widget(block, size);
    })?;

    for msg in rx {
        match msg {
            UiEvent::Resize(_, _) => terminal.draw(|f| {
                let size = f.size();
                let block = Block::default().title("Variables").borders(Borders::ALL);
                f.render_widget(block, size);
            })?,
        };
    }

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

enum UiEvent {
    Resize(u16, u16),
}

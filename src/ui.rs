use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::Stdout;
use std::sync::{Arc, Mutex};
use std::thread;
use tui::backend::CrosstermBackend;
use tui::style::Color;
use tui::style::Style;
use tui::widgets::{Block, Borders, List};
use tui::Terminal;

pub struct Ui {
    pub tx: Sender<UiEvent>,

    state: UiState,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

pub type WrappedUi = Arc<Mutex<Ui>>;

impl Ui {
    pub fn new() -> Result<WrappedUi> {
        // Prepare terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        let (tx, rx) = crossbeam_channel::unbounded();

        let ui = Arc::new(Mutex::new(Self {
            tx,

            state: UiState::default(),
            terminal,
        }));

        handle_input(ui.clone());
        handle_ui(ui.clone(), rx);

        Ok(ui)
    }

    pub fn quit(&mut self) -> Result<()> {
        // Restore terminal
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;

        Ok(())
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.quit().unwrap();
    }
}

#[derive(Default)]
struct UiState {}

pub enum UiEvent {
    Resize(u16, u16),
    NextItem,
    PrevItem,
    Quit,
}

fn handle_input(ui: WrappedUi) {
    thread::spawn(move || -> Result<()> {
        loop {
            let msg = read()?;
            let ui = ui.lock().unwrap();
            match msg {
                Event::Key(event) => match event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        ui.tx.send(UiEvent::Quit)?;
                        break;
                    }
                    KeyCode::Char('j') => ui.tx.send(UiEvent::NextItem)?,
                    KeyCode::Char('k') => ui.tx.send(UiEvent::PrevItem)?,
                    _ => (),
                },
                Event::Mouse(event) => debug!("{:?}", event),
                Event::Resize(width, height) => ui.tx.send(UiEvent::Resize(width, height))?,
            }
        }

        Ok(())
    });
}

fn handle_ui(ui: WrappedUi, rx: Receiver<UiEvent>) {
    thread::spawn(move || -> Result<()> {
        for msg in rx {
            let mut ui = ui.lock().unwrap();

            match msg {
                UiEvent::Resize(_, _) => (),
                UiEvent::NextItem => (),
                UiEvent::PrevItem => (),
                UiEvent::Quit => {
                    ui.quit()?;
                    break;
                }
            };

            ui.terminal.draw(|f| {
                let list = List::new(vec![])
                    .block(Block::default().borders(Borders::ALL).title("Variables"))
                    .highlight_style(Style::default().fg(Color::Cyan));

                let size = f.size();
                f.render_widget(list, size);
            })?;
        }

        Ok(())
    });
}

use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use crossterm::event::{read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::thread::{self, JoinHandle};
use tui::backend::CrosstermBackend;
use tui::style::Color;
use tui::style::Style;
use tui::widgets::{Block, Borders, List, ListItem, ListState};
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
                KeyCode::Char('j') => tx.send(UiEvent::NextItem)?,
                KeyCode::Char('k') => tx.send(UiEvent::PrevItem)?,
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

    let mut variables = Variables::new(vec![
        String::from("Var 1"),
        String::from("Var 2"),
        String::from("Var 3"),
        String::from("Var 4"),
        String::from("Var 5"),
        String::from("Var 6"),
    ]);
    variables.state.select(Some(0));

    for msg in rx {
        match msg {
            UiEvent::Resize(_, _) => (),
            UiEvent::NextItem => variables.next(),
            UiEvent::PrevItem => variables.previous(),
        };

        terminal.draw(|f| {
            let items: Vec<ListItem> = variables
                .list
                .iter()
                .map(|i| ListItem::new(i.as_str()).style(Style::default().fg(Color::Magenta)))
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Variables"))
                .highlight_style(Style::default().fg(Color::Cyan));

            let size = f.size();
            f.render_stateful_widget(list, size, &mut variables.state);
        })?;
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
    NextItem,
    PrevItem,
}

struct Variables {
    list: Vec<String>,
    state: ListState,
}

impl Variables {
    fn new(list: Vec<String>) -> Self {
        Self {
            list,
            state: ListState::default(),
        }
    }

    pub fn set_items(&mut self, list: Vec<String>) {
        self.list = list;
        // We reset the state as the associated list have changed. This effectively reset
        // the selection as well as the stored offset.
        self.state = ListState::default();
    }

    // Select the next item. This will not be reflected until the widget is drawn in the
    // `Terminal::draw` callback using `Frame::render_stateful_widget`.
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.list.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Select the previous item. This will not be reflected until the widget is drawn in the
    // `Terminal::draw` callback using `Frame::render_stateful_widget`.
    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.list.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    // Unselect the currently selected item if any. The implementation of `ListState` makes
    // sure that the stored offset is also reset.
    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, EventStream, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::Stdout;
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets;
use tui::widgets::{Borders, ListItem, ListState};

pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

pub struct Ui {
    pub input_stream: EventStream,

    terminal: Terminal,

    // State
    file: Vec<String>,
    file_state: ListState,
    focused: FocusedWidget,
}

impl Ui {
    pub async fn new() -> Result<Self> {
        // Prepare terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = tui::Terminal::new(backend)?;

        // TEMPORARY: Display the main test file
        let path = std::env::current_dir()?.join("test.py");
        let contents = tokio::fs::read_to_string(path).await?;
        let lines: Vec<String> = contents.lines().map(str::to_string).collect();
        let mut state = ListState::default();
        state.select(Some(0));

        Ok(Self {
            input_stream: EventStream::new(),
            terminal,

            file: lines,
            file_state: state,
            focused: FocusedWidget::SourceFile,
        })
    }

    pub fn destroy(&mut self) -> Result<()> {
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

    pub async fn handle_input(&mut self, event: crossterm::event::Event) -> Result<Order> {
        match event {
            crossterm::event::Event::Key(event) => match event.code {
                // Movement
                KeyCode::Char('g') => match self.focused {
                    FocusedWidget::SourceFile => self.file_state.select(Some(0)),
                },
                KeyCode::Char('G') => match self.focused {
                    FocusedWidget::SourceFile => self.file_state.select(Some(self.file.len() - 1)),
                },
                KeyCode::Char('j') => match self.focused {
                    FocusedWidget::SourceFile => {
                        let selected = self.file_state.selected().unwrap();
                        if selected < self.file.len() - 1 {
                            self.file_state.select(Some(selected + 1));
                        }
                    }
                },
                KeyCode::Char('k') => match self.focused {
                    FocusedWidget::SourceFile => {
                        let selected = self.file_state.selected().unwrap();
                        if selected > 0 {
                            self.file_state.select(Some(selected - 1));
                        }
                    }
                },
                // TEMPORARY:
                // KeyCode::Char('c') => socket.send("continue".to_string()).await?,
                // KeyCode::Char('i') => socket.send("in".to_string()).await?,
                // Quit
                KeyCode::Char('q') => return Ok(Order::Quit),
                _ => (),
            },
            crossterm::event::Event::Mouse(_) => (),
            crossterm::event::Event::Resize(_, _) => (),
        };

        Ok(Order::None)
    }

    pub fn draw(&mut self, _state: &crate::controller::State) -> Result<()> {
        self.terminal.draw(|f| {
            let num_width = format!("{}", self.file.len()).len();
            // TODO: Store the file content as ListItems so we don't have to convert on every render
            let lines: Vec<ListItem> = self
                .file
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>width$}  {}", i + 1, line, width = num_width))
                .map(ListItem::new)
                .collect();
            let file = widgets::List::new(lines)
                .block(
                    widgets::Block::default()
                        .title(" Source file ")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Green)),
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_stateful_widget(file, f.size(), &mut self.file_state);
        })?;

        Ok(())
    }
}

pub enum Order {
    None,
    Quit,
}

enum FocusedWidget {
    // Breakpoints,
    // CallStack,
    // DebugConsole,
    SourceFile,
    // Variables,
    // Watch,
}

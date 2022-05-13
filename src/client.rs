use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::{SinkExt, StreamExt};
use std::io::Stdout;
use std::path::{Path, PathBuf};
use tokio::net::UnixStream;
use tokio::select;
use tokio_util::codec::{Framed, LinesCodec};
use tui::backend::CrosstermBackend;
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Borders, ListItem, ListState};
use tui::{widgets, Terminal};

pub async fn run(socket_path: PathBuf) -> Result<()> {
    // TODO: Set up centralized logging on server

    // Prepare terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Set up server and input comms
    let socket = UnixStream::connect(socket_path).await.unwrap();
    let mut socket = Framed::new(socket, LinesCodec::new());
    let mut input_stream = crossterm::event::EventStream::new();

    let mut state = State::new().await?;

    // Draw UI with initial state
    draw_ui(&mut terminal, &mut state)?;

    // Main loop
    loop {
        select! {
            // User input
            Some(Ok(event)) = input_stream.next() => {
                match handle_input(&mut socket, &mut state, event).await? {
                    Order::Quit => break,
                    Order::None => () // Duh
                }
            },
            // Messages from server
            msg = socket.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        #[allow(clippy::single_match)]
                        match msg.as_str() {
                            "quit" => break,
                            _ => ()
                        }
                    }
                    Some(Err(e)) => {
                        error!("Socket error: {e}");
                    }
                    None => {
                        info!("Server disconnected");
                        break
                    }
                }
            }
        }
        // TODO: Only do this when necessary
        draw_ui(&mut terminal, &mut state)?
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

async fn handle_input(
    socket: &mut Framed<UnixStream, LinesCodec>,
    state: &mut State,
    event: crossterm::event::Event,
) -> Result<Order> {
    match event {
        crossterm::event::Event::Key(event) => match event.code {
            // Movement
            KeyCode::Char('g') => match state.focused {
                FocusedWidget::SourceFile => state.file_state.select(Some(0)),
            },
            KeyCode::Char('G') => match state.focused {
                FocusedWidget::SourceFile => state.file_state.select(Some(state.file.len() - 1)),
            },
            KeyCode::Char('j') => match state.focused {
                FocusedWidget::SourceFile => {
                    let selected = state.file_state.selected().unwrap();
                    if selected < state.file.len() - 1 {
                        state.file_state.select(Some(selected + 1));
                    }
                }
            },
            KeyCode::Char('k') => match state.focused {
                FocusedWidget::SourceFile => {
                    let selected = state.file_state.selected().unwrap();
                    if selected > 0 {
                        state.file_state.select(Some(selected - 1));
                    }
                }
            },
            // TEMPORARY: Step in
            KeyCode::Char('i') => socket.send("in".to_string()).await?,
            // Quit
            KeyCode::Char('q') => return Ok(Order::Quit),
            _ => (),
        },
        crossterm::event::Event::Mouse(_) => (),
        crossterm::event::Event::Resize(_, _) => (),
    };

    Ok(Order::None)
}

fn draw_ui(terminal: &mut Terminal<CrosstermBackend<Stdout>>, state: &mut State) -> Result<()> {
    terminal.draw(|f| {
        let num_width = format!("{}", state.file.len()).len();
        // TODO: Store the file content as ListItems so we don't have to convert on every render
        let lines: Vec<ListItem> = state
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
        f.render_stateful_widget(file, f.size(), &mut state.file_state);
    })?;

    Ok(())
}

enum Order {
    None,
    Quit,
}

struct State {
    file: Vec<String>,
    file_state: ListState,
    focused: FocusedWidget,
}

impl State {
    pub async fn new() -> Result<Self> {
        // TEMPORARY: Display the main test file
        let path = std::env::current_dir()?.join("test.py");
        let contents = tokio::fs::read_to_string(path).await?;
        let lines: Vec<String> = contents.lines().map(str::to_string).collect();
        let mut state = ListState::default();
        state.select(Some(0));
        Ok(Self {
            file: lines,
            file_state: state,
            focused: FocusedWidget::SourceFile,
        })
    }
}

enum FocusedWidget {
    // Breakpoints,
    // CallStack,
    // DebugConsole,
    SourceFile,
    // Variables,
    // Watch,
}

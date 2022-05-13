use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use futures_util::{SinkExt, StreamExt};
use std::io::Stdout;
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::select;
use tokio_util::codec::{Framed, LinesCodec};
use tui::backend::CrosstermBackend;
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

    let mut state = State::new();

    // Draw UI with initial state
    draw_ui(&mut terminal)?;

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
        draw_ui(&mut terminal)?
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
            KeyCode::Backspace => (),
            KeyCode::Enter => (),
            KeyCode::Left => (),
            KeyCode::Right => (),
            KeyCode::Up => (),
            KeyCode::Down => (),
            KeyCode::Home => (),
            KeyCode::End => (),
            KeyCode::PageUp => (),
            KeyCode::PageDown => (),
            KeyCode::Tab => (),
            KeyCode::BackTab => (),
            KeyCode::Delete => (),
            KeyCode::Insert => (),
            KeyCode::F(_) => (),
            KeyCode::Char('i') => socket.send("in".to_string()).await?,
            KeyCode::Char('q') => return Ok(Order::Quit),
            KeyCode::Char(_) => (),
            KeyCode::Null => (),
            KeyCode::Esc => (),
        },
        crossterm::event::Event::Mouse(_) => (),
        crossterm::event::Event::Resize(_, _) => (),
    };

    Ok(Order::None)
}

fn draw_ui(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    terminal.draw(|f| {
        let block = widgets::Block::default()
            .title("Pesticide")
            .borders(widgets::Borders::ALL);
        f.render_widget(block, f.size());
    })?;

    Ok(())
}

enum Order {
    None,
    Quit,
}

struct State {
    focused: FocusedWidget,
}

impl State {
    pub fn new() -> Self {
        Self {
            focused: FocusedWidget::Variables,
        }
    }
}

enum FocusedWidget {
    Breakpoints,
    CallStack,
    DebugConsole,
    SourceFile,
    Variables,
    Watch,
}

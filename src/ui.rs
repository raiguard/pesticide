use crate::controller::Action;
use crate::dap_types::*;
use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, EventStream, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use std::io::Stdout;
use tui::backend::CrosstermBackend;
use tui::style::{Color, Style};
use tui::text::{Span, Spans};
use tui::widgets;
use tui::widgets::{Borders, ListItem, ListState};

pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

pub struct Ui {
    pub input_stream: EventStream,

    terminal: Terminal,

    // Source file
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

    pub fn handle_input(&mut self, event: crossterm::event::Event) -> Result<Option<Action>> {
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
                // Adapter requests
                KeyCode::Char('i') => return Ok(Some(Action::StepIn)),
                // Quit
                KeyCode::Char('q') => return Ok(Some(Action::Quit)),
                _ => (),
            },
            crossterm::event::Event::Mouse(_) => (),
            crossterm::event::Event::Resize(_, _) => return Ok(Some(Action::Redraw)),
        };

        Ok(None)
    }

    pub fn draw(&mut self, state: &crate::controller::State) -> Result<()> {
        self.terminal.draw(|f| {
            // Source file stuff
            // let num_width = format!("{}", self.file.len()).len();
            // // TODO: Store the file content as ListItems so we don't have to convert on every render
            // let lines: Vec<ListItem> = self
            //     .file
            //     .iter()
            //     .enumerate()
            //     .map(|(i, line)| format!("{:>width$}  {}", i + 1, line, width = num_width))
            //     .map(ListItem::new)
            //     .collect();
            // let file = widgets::List::new(lines)
            //     .block(
            //         widgets::Block::default()
            //             .title(" Source file ")
            //             .borders(Borders::ALL)
            //             .style(Style::default().fg(Color::Green)),
            //     )
            //     .style(Style::default().fg(Color::White))
            //     .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            // f.render_stateful_widget(file, f.size(), &mut self.file_state);

            // Stack frames
            let mut stack_frames: Vec<ListItem> = vec![];
            for thread in &state.threads {
                if let Some(frames) = state.stack_frames.get(&thread.id) {
                    // Thread header
                    let reason = state
                        .stopped_threads
                        .get(&thread.id)
                        .or(if state.all_threads_stopped {
                            Some(&StoppedReason::Pause)
                        } else {
                            None
                        })
                        .map(|reason| {
                            match reason {
                                StoppedReason::Step => "Stopped on step",
                                StoppedReason::Breakpoint => "Paused on breakpoint",
                                StoppedReason::Exception => "Paused on exception",
                                StoppedReason::Pause => "Paused",
                                StoppedReason::Entry => "Paused on entry",
                                StoppedReason::Goto => "Paused on goto",
                                StoppedReason::FunctionBreakpoint => {
                                    "Paused on function breakpoint"
                                }
                                StoppedReason::DataBreakpoint => "Paused on data breakpoint",
                                StoppedReason::InstructionBreakpoint => {
                                    "Paused on instruction breakpoint"
                                }
                            }
                            .to_string()
                        })
                        .unwrap_or_else(|| String::from("Running"));
                    stack_frames.push(ListItem::new(Spans::from(vec![
                        Span::styled(
                            format!("▼ {:<20}", thread.name),
                            Style::default().fg(Color::Blue),
                        ),
                        Span::styled(reason.to_string(), Style::default().fg(Color::White)),
                    ])));

                    // Stack frames within thread
                    for frame in frames {
                        let mut line = vec![Span::raw(format!("  {:<20}", frame.name.clone()))];
                        // Source info
                        if let Some(source) = &frame.source {
                            let name = if let Some(name) = &source.name {
                                name.clone()
                            } else if let Some(path) = &source.path {
                                path.file_name()
                                    .and_then(|name| name.to_str())
                                    .map(|name| name.to_string())
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            line.push(Span::styled(
                                format!("{}:{}:{}", name, frame.line, frame.column),
                                Style::default().fg(Color::Cyan),
                            ));
                        }
                        stack_frames.push(ListItem::new(Spans::from(line)));
                    }
                }
            }

            let stack_frames_list = widgets::List::new(stack_frames).block(
                widgets::Block::default()
                    .title("Call stack")
                    .borders(Borders::ALL),
            );

            f.render_widget(stack_frames_list, f.size());
        })?;

        Ok(())
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

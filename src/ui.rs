use crate::controller::Action;
use crate::dap_types::*;
use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, EventStream, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use itertools::Itertools;
use std::collections::HashSet;
use std::io::Stdout;
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Corner, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{self, Block, List};
use tui::widgets::{Borders, ListItem, ListState};

pub type Terminal = tui::Terminal<CrosstermBackend<Stdout>>;

pub struct Ui {
    pub input_stream: EventStream,

    terminal: Terminal,

    call_stack_list: ListStateWithData<CallStackItemKind>,
    expanded_threads: HashSet<u32>,

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

            call_stack_list: ListStateWithData::new(),
            expanded_threads: HashSet::new(),

            file: lines,
            file_state: state,
            focused: FocusedWidget::CallStack,
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

    pub fn handle_input(
        &mut self,
        state: &mut crate::controller::State,
        event: crossterm::event::Event,
    ) -> Result<Vec<Action>> {
        let mut actions = vec![];
        match event {
            crossterm::event::Event::Key(event) => match event.code {
                KeyCode::Char(' ') => match self.focused {
                    FocusedWidget::CallStack => {
                        if let Some(focused_line) = self.call_stack_list.selected_item() {
                            match focused_line {
                                CallStackItemKind::Thread(thread_id) => {
                                    if self.expanded_threads.contains(thread_id) {
                                        self.expanded_threads.remove(thread_id);
                                    } else {
                                        self.expanded_threads.insert(*thread_id);
                                        if state.stack_frames.get(thread_id).is_none() {
                                            actions.push(Action::Request(Request::StackTrace(
                                                StackTraceArgs {
                                                    thread_id: *thread_id,
                                                    start_frame: None,
                                                    levels: None,
                                                    format: None,
                                                },
                                            )));
                                        }
                                    }
                                }
                                CallStackItemKind::StackFrame(thread_id, frame_id) => {
                                    state.current_thread = *thread_id;
                                    state.current_stack_frame = *frame_id
                                }
                            }
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::SourceFile => todo!(),
                },
                // Movement
                KeyCode::Char('g') => match self.focused {
                    FocusedWidget::CallStack => {
                        self.call_stack_list.select(0);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::SourceFile => self.file_state.select(Some(0)),
                },
                KeyCode::Char('G') => match self.focused {
                    FocusedWidget::CallStack => {
                        self.call_stack_list.select(self.call_stack_list.len() - 1);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::SourceFile => self.file_state.select(Some(self.file.len() - 1)),
                },
                KeyCode::Char('j') => match self.focused {
                    FocusedWidget::CallStack => {
                        if let Some(i) = self.call_stack_list.selected() {
                            self.call_stack_list.select(i + 1);
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::SourceFile => {
                        let selected = self.file_state.selected().unwrap();
                        if selected < self.file.len() - 1 {
                            self.file_state.select(Some(selected + 1));
                        }
                    }
                },
                KeyCode::Char('k') => match self.focused {
                    FocusedWidget::CallStack => {
                        if let Some(i) = self.call_stack_list.selected() {
                            if i > 0 {
                                self.call_stack_list.select(i - 1);
                                actions.push(Action::Redraw);
                            }
                        }
                    }
                    FocusedWidget::SourceFile => {
                        let selected = self.file_state.selected().unwrap();
                        if selected > 0 {
                            self.file_state.select(Some(selected - 1));
                        }
                    }
                },
                // Adapter requests
                KeyCode::Char('c') => {
                    actions.push(Action::Request(Request::Continue(ContinueArgs {
                        thread_id: state.current_thread,
                        single_thread: true,
                    })))
                }
                KeyCode::Char('i') => actions.push(Action::Request(Request::StepIn(StepInArgs {
                    thread_id: state.current_thread,
                    single_thread: true,
                    target_id: None,
                    granularity: SteppingGranularity::Line,
                }))),
                // Quit
                KeyCode::Char('q') => actions.push(Action::Quit),
                _ => (),
            },
            crossterm::event::Event::Mouse(_) => (),
            crossterm::event::Event::Resize(_, _) => actions.push(Action::Redraw),
        };

        Ok(actions)
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

            // Layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(30),
                        Constraint::Min(40),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // Variables
            let mut variables = vec![];
            if let Some(scopes) = state.scopes.get(&state.current_stack_frame) {
                for scope in scopes {
                    variables.push(ListItem::new(format!("{} {}", "▼", scope.name)));
                }
            }
            f.render_widget(
                List::new(variables)
                    .block(Block::default().title("Variables").borders(Borders::ALL)),
                chunks[0],
            );
            // // Watches
            // f.render_widget(
            //     Block::default().title("Watch").borders(Borders::ALL),
            //     chunks[1],
            // );

            // Stack frames
            let mut stack_frames = vec![];
            let mut stacktrace_list = vec![];
            for thread in &state.threads {
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
                            StoppedReason::Step => "Paused on step",
                            StoppedReason::Breakpoint => "Paused on breakpoint",
                            StoppedReason::Exception => "Paused on exception",
                            StoppedReason::Pause => "Paused",
                            StoppedReason::Entry => "Paused on entry",
                            StoppedReason::Goto => "Paused on goto",
                            StoppedReason::FunctionBreakpoint => "Paused on function breakpoint",
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
                        format!(
                            "{} {:<20}",
                            if self.expanded_threads.contains(&thread.id) {
                                "▼"
                            } else {
                                "▶"
                            },
                            thread.name
                        ),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::styled(reason.to_string(), Style::default().fg(Color::White)),
                ])));
                stacktrace_list.push(CallStackItemKind::Thread(thread.id));

                if self.expanded_threads.contains(&thread.id) {
                    if let Some(frames) = state.stack_frames.get(&thread.id) {
                        // Stack frames within thread
                        for frame in frames {
                            let mut line = vec![
                                Span::styled(
                                    if state.current_stack_frame == frame.id {
                                        "*"
                                    } else {
                                        " "
                                    }
                                    .to_string(),
                                    Style::default().fg(Color::Green),
                                ),
                                Span::styled(
                                    format!(" {:<20}", frame.name),
                                    Style::default().fg(Color::White),
                                ),
                            ];
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
                            stacktrace_list
                                .push(CallStackItemKind::StackFrame(thread.id, frame.id));
                        }
                    }
                }
            }
            self.call_stack_list.update_items(stacktrace_list);
            let stack_frames_list = List::new(stack_frames)
                .block(
                    Block::default()
                        .title("Call stack")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(
                            if let FocusedWidget::CallStack = self.focused {
                                Color::Green
                            } else {
                                Color::White
                            },
                        )),
                )
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_stateful_widget(
                stack_frames_list,
                chunks[1],
                self.call_stack_list.get_internal_state(),
            );

            // // Breakpoints
            // f.render_widget(
            //     Block::default().title("Breakpoints").borders(Borders::ALL),
            //     chunks[3],
            // );

            // Debugee console
            let console_list = widgets::List::new(
                state
                    .console
                    .iter()
                    .rev()
                    .cloned()
                    .map(ListItem::new)
                    .collect::<Vec<ListItem>>(),
            )
            .start_corner(Corner::BottomLeft)
            .block(
                widgets::Block::default()
                    .title("Debug console")
                    .borders(Borders::ALL),
            );
            f.render_widget(console_list, chunks[2]);
        })?;

        Ok(())
    }
}

enum FocusedWidget {
    // Breakpoints,
    CallStack,
    // DebugConsole,
    SourceFile,
    // Variables,
    // Watch,
}

#[derive(PartialEq)]
enum CallStackItemKind {
    // Thread ID
    Thread(u32),
    // Thread ID and stack frame ID
    StackFrame(u32, u32),
}

/// Wrapper for `ListState` that contains associated data.
///
/// The default `ListState` does not associate with the data in the list. This
/// wrapper stores the state and representations of the data in each line.
///
/// Intended use is for the stored data to "point to" the actual data. For
/// example, store a list of `usize` that correspond to variables references,
/// instead of storing the actual variables references.
struct ListStateWithData<T> {
    items: Vec<T>,
    state: ListState,
}

impl<T: PartialEq> ListStateWithData<T> {
    fn new() -> Self {
        Self {
            items: vec![],
            state: ListState::default(),
        }
    }

    /// Get the internal state object, for use with `render_stateful_widget`.
    fn get_internal_state(&mut self) -> &mut ListState {
        &mut self.state
    }

    /// The length of the stored data.
    fn len(&self) -> usize {
        self.items.len()
    }

    /// Select the given index, if it exists.
    fn select(&mut self, i: usize) {
        if self.items.get(i).is_some() {
            self.state.select(Some(i));
        }
    }

    /// Get the currently selected index.
    fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Select the given item, if it exists in the items list.
    fn select_item(&mut self, item: T) {
        if let Some(i) = self
            .items
            .iter()
            .find_position(|stored| **stored == item)
            .map(|(i, _)| i)
        {
            self.state.select(Some(i));
        }
    }

    /// Get the currently selected item, if any.
    fn selected_item(&self) -> Option<&T> {
        self.state.selected().and_then(|i| self.items.get(i))
    }

    /// Update stored data and ensure the selected index is valid.
    fn update_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.state
            .select(match (self.items.len(), self.state.selected()) {
                (0, _) => None,
                (_, None) => Some(0),
                (len, Some(i)) => Some(std::cmp::min(len - 1, i)),
            });
    }
}

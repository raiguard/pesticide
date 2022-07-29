use crate::server::Action;
use crate::dap::*;
use anyhow::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, EventStream, KeyCode};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use itertools::Itertools;
use std::collections::HashSet;
use std::io::Stdout;
use std::path::PathBuf;
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
    focused: FocusedWidget,

    // Call stack
    call_stack_list: ListStateWithData<CallStackItemKind>,
    expanded_threads: HashSet<i64>,

    // Variables
    variables_list: ListStateWithData<VariablesItemKind>,
    expanded_variables: HashSet<VariablesItemKind>,
}

impl Ui {
    pub async fn new() -> Result<Self> {
        // Prepare terminal
        enable_raw_mode()?;
        let mut stdout = std::io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = tui::Terminal::new(backend)?;

        Ok(Self {
            input_stream: EventStream::new(),
            terminal,

            call_stack_list: ListStateWithData::new(),
            expanded_threads: HashSet::new(),

            variables_list: ListStateWithData::new(),
            expanded_variables: HashSet::new(),

            focused: FocusedWidget::Variables,
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
        state: &mut crate::server::State,
        event: crossterm::event::Event,
    ) -> Result<Vec<Action>> {
        let mut actions = vec![];
        match event {
            crossterm::event::Event::Key(event) => match event.code {
                KeyCode::Char(' ') => match self.focused {
                    FocusedWidget::Variables => {
                        if let Some(focused_line) = self.variables_list.selected_item() {
                            if self.expanded_variables.contains(focused_line) {
                                self.expanded_variables.remove(focused_line);
                            } else {
                                self.expanded_variables.insert(focused_line.clone());
                                match focused_line {
                                    VariablesItemKind::Scope(_, reference) => {
                                        if state.variables.get(reference).is_none() {
                                            actions.push(Action::Request(
                                                RequestArguments::variables(VariablesArguments {
                                                    variables_reference: *reference,
                                                    filter: None,
                                                    start: None,
                                                    count: None,
                                                    format: None,
                                                }),
                                            ))
                                        }
                                    }
                                    VariablesItemKind::Variable(parent_ref, index) => {
                                        if let Some(variable) = state
                                            .variables
                                            .get(parent_ref)
                                            .and_then(|variables| variables.get(*index as usize))
                                        {
                                            if variable.variables_reference > 0
                                                && !state
                                                    .variables
                                                    .contains_key(&variable.variables_reference)
                                            {
                                                actions.push(Action::Request(
                                                    RequestArguments::variables(
                                                        VariablesArguments {
                                                            variables_reference: variable
                                                                .variables_reference,
                                                            filter: None,
                                                            start: None,
                                                            count: None,
                                                            format: None,
                                                        },
                                                    ),
                                                ))
                                            }
                                        }
                                    }
                                }
                            }
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::CallStack => {
                        if let Some(focused_line) = self.call_stack_list.selected_item() {
                            match focused_line {
                                CallStackItemKind::Thread(thread_id) => {
                                    if self.expanded_threads.contains(thread_id) {
                                        self.expanded_threads.remove(thread_id);
                                    } else {
                                        self.expanded_threads.insert(*thread_id);
                                        if state.stack_frames.get(thread_id).is_none() {
                                            actions.push(Action::Request(
                                                RequestArguments::stackTrace(StackTraceArguments {
                                                    thread_id: *thread_id,
                                                    start_frame: None,
                                                    levels: None,
                                                    format: None,
                                                }),
                                            ));
                                        }
                                    }
                                }
                                CallStackItemKind::StackFrame(thread_id, frame_id) => {
                                    state.current_thread = *thread_id;
                                    state.current_stack_frame = *frame_id;
                                    actions.push(Action::Jump);
                                }
                            }
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::DebugConsole => (),
                },
                // Focused view
                KeyCode::Char('h') => {
                    self.focused = match self.focused {
                        FocusedWidget::CallStack => FocusedWidget::Variables,
                        FocusedWidget::DebugConsole => FocusedWidget::CallStack,
                        FocusedWidget::Variables => FocusedWidget::DebugConsole,
                    };
                    actions.push(Action::Redraw);
                }
                KeyCode::Char('l') => {
                    self.focused = match self.focused {
                        FocusedWidget::CallStack => FocusedWidget::DebugConsole,
                        FocusedWidget::DebugConsole => FocusedWidget::Variables,
                        FocusedWidget::Variables => FocusedWidget::CallStack,
                    };
                    actions.push(Action::Redraw);
                }
                // Movement
                KeyCode::Char('g') => match self.focused {
                    FocusedWidget::Variables => {
                        self.variables_list.select(0);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::CallStack => {
                        self.call_stack_list.select(0);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::DebugConsole => (),
                },
                KeyCode::Char('G') => match self.focused {
                    FocusedWidget::Variables => {
                        self.variables_list.select(self.variables_list.len() - 1);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::CallStack => {
                        self.call_stack_list.select(self.call_stack_list.len() - 1);
                        actions.push(Action::Redraw);
                    }
                    FocusedWidget::DebugConsole => (),
                },
                KeyCode::Char('j') => match self.focused {
                    FocusedWidget::Variables => {
                        if let Some(i) = self.variables_list.selected() {
                            self.variables_list.select(i + 1);
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::CallStack => {
                        if let Some(i) = self.call_stack_list.selected() {
                            self.call_stack_list.select(i + 1);
                            actions.push(Action::Redraw);
                        }
                    }
                    FocusedWidget::DebugConsole => (),
                },
                KeyCode::Char('k') => match self.focused {
                    FocusedWidget::Variables => {
                        if let Some(i) = self.variables_list.selected() {
                            if i > 0 {
                                self.variables_list.select(i - 1);
                                actions.push(Action::Redraw);
                            }
                        }
                    }
                    FocusedWidget::CallStack => {
                        if let Some(i) = self.call_stack_list.selected() {
                            if i > 0 {
                                self.call_stack_list.select(i - 1);
                                actions.push(Action::Redraw);
                            }
                        }
                    }
                    FocusedWidget::DebugConsole => (),
                },
                // Adapter requests
                KeyCode::Char('c') => actions.push(Action::Request(RequestArguments::continue_(
                    ContinueArguments {
                        thread_id: state.current_thread,
                        single_thread: Some(true),
                    },
                ))),
                KeyCode::Char('i') => {
                    actions.push(Action::Request(RequestArguments::stepIn(StepInArguments {
                        thread_id: state.current_thread,
                        single_thread: Some(true),
                        target_id: None,
                        granularity: Some(SteppingGranularity::Line),
                    })))
                }
                // Quit
                KeyCode::Char('q') => actions.push(Action::Quit),
                _ => (),
            },
            crossterm::event::Event::Mouse(_) => (),
            crossterm::event::Event::Resize(_, _) => actions.push(Action::Redraw),
        };

        Ok(actions)
    }

    pub fn draw(&mut self, state: &crate::server::State) -> Result<()> {
        // Variables
        let mut variables_disp = vec![];
        let mut variables_list = vec![];
        if let Some(scopes) = state.scopes.get(&state.current_stack_frame) {
            for (scope_index, scope) in scopes.iter().enumerate() {
                let scope_ident =
                    VariablesItemKind::Scope(scope_index as i64, scope.variables_reference);
                let scope_expanded = self.expanded_variables.contains(&scope_ident);

                variables_disp.push(ListItem::new(Span::styled(
                    format!("{} {}", if scope_expanded { "▼" } else { "▶" }, scope.name),
                    Style::default().fg(Color::White),
                )));
                variables_list.push(scope_ident);

                if scope_expanded {
                    walk_variables(
                        self,
                        state,
                        &mut variables_disp,
                        &mut variables_list,
                        (1, scope.variables_reference),
                    );
                }
            }
        }
        self.variables_list.update_items(variables_list);
        let variables_list = List::new(variables_disp)
            .block(
                Block::default()
                    .title("Variables")
                    .borders(Borders::ALL)
                    .style(
                        Style::default().fg(if let FocusedWidget::Variables = self.focused {
                            Color::Green
                        } else {
                            Color::White
                        }),
                    ),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        self.terminal.draw(|f| {
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

            f.render_stateful_widget(
                variables_list,
                chunks[0],
                self.variables_list.get_internal_state(),
            );

            // Stack frames
            let mut stack_frames = vec![];
            let mut stacktrace_list = vec![];
            for thread in &state.threads {
                // Thread header
                let reason = state
                    .stopped_threads
                    .get(&thread.id)
                    .cloned()
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
                    Span::styled(reason, Style::default().fg(Color::White)),
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
                                    let path = PathBuf::from(path);
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

fn walk_variables(
    ui: &Ui,
    state: &crate::server::State,
    variables_disp: &mut Vec<ListItem>,
    variables_list: &mut Vec<VariablesItemKind>,
    (indent, variables_reference): (usize, i64),
) {
    if let Some(variables) = state.variables.get(&variables_reference) {
        for (i, variable) in variables.iter().enumerate() {
            let has_children = variable.variables_reference > 0;
            let expanded = ui
                .expanded_variables
                .contains(&VariablesItemKind::Variable(variables_reference, i as i64));
            variables_disp.push(ListItem::new(Spans::from(vec![
                Span::styled(
                    format!(
                        "{:>indent$}{}{}: ",
                        "",
                        match (has_children, expanded) {
                            (true, false) => "▶ ",
                            (true, true) => "▼ ",
                            _ => "",
                        },
                        variable.name,
                        indent = indent * 2,
                    ),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(variable.value.clone(), Style::default().fg(Color::White)),
            ])));
            variables_list.push(VariablesItemKind::Variable(variables_reference, i as i64));

            if has_children && expanded {
                walk_variables(
                    ui,
                    state,
                    variables_disp,
                    variables_list,
                    (indent + 1, variable.variables_reference),
                );
            }
        }
    }
}

enum FocusedWidget {
    // Breakpoints,
    CallStack,
    DebugConsole,
    // SourceFile,
    Variables,
}

#[derive(Clone, Eq, PartialEq, Hash)]
enum VariablesItemKind {
    // Scope index and variables reference
    Scope(i64, i64),
    // Parent variables reference and variable index
    Variable(i64, i64),
}

#[derive(PartialEq)]
enum CallStackItemKind {
    // Thread ID
    Thread(i64),
    // Thread ID and stack frame ID
    StackFrame(i64, i64),
}

/// Wrapper for `ListState` that contains associated data.
///
/// The default `ListState` does not associate with the data in the list. This
/// wrapper stores the state and representations of the data in each line.
///
/// Intended use is for the stored data to "point to" the actual data. For
/// example, store a list of `i64` that correspond to variables references,
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

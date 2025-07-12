//! # Terminal User Interface Module
//!
//! This module provides a rich terminal user interface for grok-code using ratatui.
//! It offers better visualization for:
//! - Chat conversations with syntax highlighting
//! - Code diffs with side-by-side comparison
//! - Tool execution progress
//! - File tree navigation

pub mod diff;

use crate::api::Message;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{error::Error, io, time::Duration};

/// TUI application state
pub struct TuiApp {
    /// Chat messages
    messages: Vec<UiMessage>,
    /// Current input buffer
    input: String,
    /// Scroll position for messages
    scroll: u16,
    /// Whether the app should quit
    should_quit: bool,
    /// Current mode
    mode: AppMode,
    /// Status message
    status: String,
}

/// UI representation of a message
#[derive(Clone)]
struct UiMessage {
    role: String,
    content: String,
    timestamp: String,
    tool_calls: Vec<String>,
}

/// Application mode
#[derive(Clone, Copy, PartialEq)]
enum AppMode {
    Normal,
    Input,
    ScrollingMessages,
}

impl TuiApp {
    /// Create a new TUI application
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            scroll: 0,
            should_quit: false,
            mode: AppMode::Input,
            status: "Ready. Press ? for help, Esc to toggle modes, Ctrl-C to quit.".to_string(),
        }
    }

    /// Add a message to the chat
    pub fn add_message(&mut self, message: &Message) {
        let ui_msg = UiMessage {
            role: message.role.clone(),
            content: message.content.clone().unwrap_or_default(),
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            tool_calls: message
                .tool_calls
                .as_ref()
                .map(|calls| {
                    calls
                        .iter()
                        .map(|call| format!("🔧 {}", call.function.name))
                        .collect()
                })
                .unwrap_or_default(),
        };
        self.messages.push(ui_msg);

        // Auto-scroll to bottom when new message arrives
        self.scroll = self.messages.len().saturating_sub(1) as u16;
    }

    /// Get the current input
    pub fn get_input(&self) -> &str {
        &self.input
    }

    /// Clear the input buffer
    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Run the TUI event loop
    pub async fn run<B: Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> Result<Option<String>, Box<dyn Error>> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) => {
                        if let Some(input) = self.handle_key_event(key)? {
                            return Ok(Some(input));
                        }
                    }
                    Event::Mouse(_) => {}
                    Event::Resize(_, _) => {}
                    _ => {}
                }
            }

            if self.should_quit {
                return Ok(None);
            }
        }
    }

    /// Handle key events
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<String>, Box<dyn Error>> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key),
            AppMode::Input => self.handle_input_mode(key),
            AppMode::ScrollingMessages => self.handle_scroll_mode(key),
        }
    }

    /// Handle keys in normal mode
    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<Option<String>, Box<dyn Error>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            (KeyCode::Char('i'), _) => {
                self.mode = AppMode::Input;
                self.status =
                    "Input mode - Type your message, Enter to send, Esc to cancel".to_string();
            }
            (KeyCode::Char('s'), _) => {
                self.mode = AppMode::ScrollingMessages;
                self.status = "Scroll mode - Use j/k or arrows to scroll, Esc to exit".to_string();
            }
            (KeyCode::Char('?'), _) => {
                self.show_help();
            }
            _ => {}
        }
        Ok(None)
    }

    /// Handle keys in input mode
    fn handle_input_mode(&mut self, key: KeyEvent) -> Result<Option<String>, Box<dyn Error>> {
        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    let input = self.input.clone();
                    self.clear_input();
                    self.mode = AppMode::Normal;
                    self.status = "Message sent. Processing...".to_string();
                    return Ok(Some(input));
                }
            }
            KeyCode::Char(c) => {
                self.input.push(c);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.status = "Normal mode - Press i to input, s to scroll, ? for help".to_string();
            }
            _ => {}
        }
        Ok(None)
    }

    /// Handle keys in scroll mode
    fn handle_scroll_mode(&mut self, key: KeyEvent) -> Result<Option<String>, Box<dyn Error>> {
        let max_scroll = self.messages.len().saturating_sub(1) as u16;

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = (self.scroll + 1).min(max_scroll);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            KeyCode::PageDown => {
                self.scroll = (self.scroll + 10).min(max_scroll);
            }
            KeyCode::PageUp => {
                self.scroll = self.scroll.saturating_sub(10);
            }
            KeyCode::Home => {
                self.scroll = 0;
            }
            KeyCode::End => {
                self.scroll = max_scroll;
            }
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.status = "Normal mode - Press i to input, s to scroll, ? for help".to_string();
            }
            _ => {}
        }
        Ok(None)
    }

    /// Show help message
    fn show_help(&mut self) {
        self.status =
            "Help: i=input s=scroll j/k=up/down Enter=send Esc=cancel Ctrl-C=quit".to_string();
    }

    /// Draw the UI
    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),    // Messages area
                Constraint::Length(3), // Input area
                Constraint::Length(1), // Status bar
            ])
            .split(f.area());

        self.draw_messages(f, chunks[0]);
        self.draw_input(f, chunks[1]);
        self.draw_status(f, chunks[2]);
    }

    /// Draw the messages area
    fn draw_messages(&mut self, f: &mut Frame, area: Rect) {
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .flat_map(|msg| {
                let mut items = vec![];

                // Role and timestamp
                let header = format!("[{}] {}", msg.timestamp, msg.role);
                let style = match msg.role.as_str() {
                    "user" => Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                    "assistant" => Style::default()
                        .fg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                    "system" => Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                    _ => Style::default().fg(Color::Gray),
                };
                items.push(ListItem::new(header).style(style));

                // Content
                for line in msg.content.lines() {
                    items.push(ListItem::new(format!("  {line}")));
                }

                // Tool calls
                for tool in &msg.tool_calls {
                    items.push(
                        ListItem::new(format!("  {tool}")).style(Style::default().fg(Color::Cyan)),
                    );
                }

                // Empty line between messages
                items.push(ListItem::new(""));

                items
            })
            .collect();

        let messages_list = List::new(messages)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Chat Messages ")
                    .border_style(match self.mode {
                        AppMode::ScrollingMessages => Style::default().fg(Color::Yellow),
                        _ => Style::default(),
                    }),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_widget(messages_list, area);
    }

    /// Draw the input area
    fn draw_input(&self, f: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Input ")
                    .border_style(match self.mode {
                        AppMode::Input => Style::default().fg(Color::Green),
                        _ => Style::default(),
                    }),
            );

        f.render_widget(input, area);

        // Show cursor in input mode
        if self.mode == AppMode::Input {
            f.set_cursor_position((area.x + self.input.len() as u16 + 1, area.y + 1))
        }
    }

    /// Draw the status bar
    fn draw_status(&self, f: &mut Frame, area: Rect) {
        let status = Paragraph::new(self.status.as_str())
            .style(Style::default().fg(Color::Gray).bg(Color::Black))
            .alignment(Alignment::Left);

        f.render_widget(status, area);
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal
pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Run a simple TUI demo
pub async fn run_tui_demo() -> Result<(), Box<dyn Error>> {
    let mut terminal = init_terminal()?;
    let mut app = TuiApp::new();

    // Add some demo messages
    app.add_message(&Message {
        role: "system".to_string(),
        content: Some("Welcome to Grok Code TUI mode!".to_string()),
        tool_calls: None,
        tool_call_id: None,
    });

    app.add_message(&Message {
        role: "user".to_string(),
        content: Some("Show me the main.rs file".to_string()),
        tool_calls: None,
        tool_call_id: None,
    });

    app.add_message(&Message {
        role: "assistant".to_string(),
        content: Some("I'll read the main.rs file for you.".to_string()),
        tool_calls: Some(vec![]),
        tool_call_id: None,
    });

    let result = app.run(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    if let Ok(Some(input)) = result {
        println!("User input: {input}");
    }

    Ok(())
}

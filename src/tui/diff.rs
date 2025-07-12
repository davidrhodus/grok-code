//! Code diff visualization for the TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

/// Represents a diff hunk
#[derive(Clone)]
pub struct DiffHunk {
    pub file_path: String,
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

/// Represents a single line in a diff
#[derive(Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
}

/// Type of diff line
#[derive(Clone, Copy, PartialEq)]
pub enum DiffLineType {
    Context,
    Added,
    Removed,
    Header,
}

/// A widget for displaying code diffs
pub struct DiffView {
    hunks: Vec<DiffHunk>,
    scroll: usize,
}

impl DiffView {
    /// Create a new diff view
    pub fn new(hunks: Vec<DiffHunk>) -> Self {
        Self { hunks, scroll: 0 }
    }

    /// Render the diff view
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Content
            ])
            .split(area);

        // Render header
        let header = Paragraph::new("Code Diff View - Use j/k to scroll, q to close")
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Render diff content
        self.render_diff_content(f, chunks[1]);
    }

    /// Render the diff content
    fn render_diff_content(&self, f: &mut Frame, area: Rect) {
        let mut items: Vec<ListItem> = Vec::new();

        for hunk in &self.hunks {
            // File header
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("=== {} ===", hunk.file_path),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ])));

            // Hunk header
            items.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!(
                        "@@ -{},{} +{},{} @@",
                        hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
                    ),
                    Style::default().fg(Color::Magenta),
                ),
            ])));

            // Diff lines
            for line in &hunk.lines {
                let (prefix, style) = match line.line_type {
                    DiffLineType::Context => (" ", Style::default()),
                    DiffLineType::Added => ("+", Style::default().fg(Color::Green)),
                    DiffLineType::Removed => ("-", Style::default().fg(Color::Red)),
                    DiffLineType::Header => ("", Style::default().fg(Color::Yellow)),
                };

                let line_nums = format!(
                    "{:>4} {:>4} ",
                    line.old_line_num.map_or(String::new(), |n| n.to_string()),
                    line.new_line_num.map_or(String::new(), |n| n.to_string()),
                );

                items.push(ListItem::new(Line::from(vec![
                    Span::styled(line_nums, Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}{}", prefix, line.content), style),
                ])));
            }

            // Empty line between hunks
            items.push(ListItem::new(""));
        }

        let diff_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Diff Content "),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(diff_list, area);
    }

    /// Scroll up
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    /// Scroll down
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_add(amount);
    }
}

/// Parse a unified diff into hunks
pub fn parse_unified_diff(diff_text: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_line = 0;
    let mut new_line = 0;

    for line in diff_text.lines() {
        if line.starts_with("diff --git") {
            // New file
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
        } else if line.starts_with("---") || line.starts_with("+++") {
            // File headers - extract filename
            if line.starts_with("+++") {
                if let Some(path) = line.strip_prefix("+++ b/") {
                    if current_hunk.is_none() {
                        current_hunk = Some(DiffHunk {
                            file_path: path.to_string(),
                            old_start: 0,
                            old_count: 0,
                            new_start: 0,
                            new_count: 0,
                            lines: Vec::new(),
                        });
                    }
                }
            }
        } else if line.starts_with("@@") {
            // Hunk header
            if let Some(ref mut hunk) = current_hunk {
                // Parse @@ -old_start,old_count +new_start,new_count @@
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    if let Some(old_info) = parts[1].strip_prefix('-') {
                        let old_parts: Vec<&str> = old_info.split(',').collect();
                        old_line = old_parts[0].parse().unwrap_or(1);
                        hunk.old_start = old_line;
                        hunk.old_count = old_parts
                            .get(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1);
                    }
                    if let Some(new_info) = parts[2].strip_prefix('+') {
                        let new_parts: Vec<&str> = new_info.split(',').collect();
                        new_line = new_parts[0].parse().unwrap_or(1);
                        hunk.new_start = new_line;
                        hunk.new_count = new_parts
                            .get(1)
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(1);
                    }
                }

                hunk.lines.push(DiffLine {
                    line_type: DiffLineType::Header,
                    content: line.to_string(),
                    old_line_num: None,
                    new_line_num: None,
                });
            }
        } else if let Some(ref mut hunk) = current_hunk {
            // Diff content
            let (line_type, content) = if let Some(content) = line.strip_prefix('+') {
                (DiffLineType::Added, content)
            } else if let Some(content) = line.strip_prefix('-') {
                (DiffLineType::Removed, content)
            } else {
                (DiffLineType::Context, line.strip_prefix(' ').unwrap_or(line))
            };

            let (old_num, new_num) = match line_type {
                DiffLineType::Added => {
                    let n = Some(new_line);
                    new_line += 1;
                    (None, n)
                }
                DiffLineType::Removed => {
                    let n = Some(old_line);
                    old_line += 1;
                    (n, None)
                }
                DiffLineType::Context => {
                    let o = Some(old_line);
                    let n = Some(new_line);
                    old_line += 1;
                    new_line += 1;
                    (o, n)
                }
                _ => (None, None),
            };

            hunk.lines.push(DiffLine {
                line_type,
                content: content.to_string(),
                old_line_num: old_num,
                new_line_num: new_num,
            });
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    hunks
} 
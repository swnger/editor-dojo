use std::io;
use std::collections::HashSet;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};

use crate::domain::{Challenge, Progress};

#[derive(Debug, Clone, Copy, PartialEq)]
enum FilterMode {
    All,
    Incomplete,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayMode {
    List,
    FilterPanel,
}

pub struct ChallengeListScreen {
    all_challenges: Vec<Challenge>,
    filtered_challenges: Vec<usize>, // Indices into all_challenges
    progress: Option<Progress>,
    selected_index: usize,
    filter_mode: FilterMode,
    tag_filters: HashSet<String>,
    display_mode: DisplayMode,
    filter_panel_selected: usize,
    available_tags: Vec<String>,
}

impl ChallengeListScreen {
    pub fn new(challenges: Vec<Challenge>) -> Self {
        let filtered_challenges: Vec<usize> = (0..challenges.len()).collect();
        let available_tags = Self::extract_all_tags(&challenges);

        Self {
            all_challenges: challenges,
            filtered_challenges,
            progress: None,
            selected_index: 0,
            filter_mode: FilterMode::All,
            tag_filters: HashSet::new(),
            display_mode: DisplayMode::List,
            filter_panel_selected: 0,
            available_tags,
        }
    }

    pub fn with_progress(mut self, progress: Progress) -> Self {
        self.progress = Some(progress);
        self
    }

    /// Extract all unique tags from challenges, sorted
    fn extract_all_tags(challenges: &[Challenge]) -> Vec<String> {
        let mut tags: HashSet<String> = HashSet::new();
        for challenge in challenges {
            for tag in challenge.tags() {
                tags.insert(tag.clone());
            }
        }
        let mut tag_vec: Vec<String> = tags.into_iter().collect();
        tag_vec.sort();
        tag_vec
    }

    /// Apply current filters and update filtered_challenges
    fn apply_filters(&mut self) {
        self.filtered_challenges = self.all_challenges
            .iter()
            .enumerate()
            .filter(|(_, challenge)| {
                // Filter by completion status
                let passes_completion_filter = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::Incomplete => {
                        if let Some(ref progress) = self.progress {
                            progress
                                .get_challenge_stats(challenge.id())
                                .is_none_or(|stats| !stats.is_completed())
                        } else {
                            true
                        }
                    }
                    FilterMode::Completed => {
                        if let Some(ref progress) = self.progress {
                            progress
                                .get_challenge_stats(challenge.id())
                                .is_some_and(|stats| stats.is_completed())
                        } else {
                            false
                        }
                    }
                };

                // Filter by tags (if any tags selected, challenge must have at least one)
                let passes_tag_filter = if self.tag_filters.is_empty() {
                    true
                } else {
                    challenge.tags().iter().any(|tag| self.tag_filters.contains(tag))
                };

                passes_completion_filter && passes_tag_filter
            })
            .map(|(idx, _)| idx)
            .collect();

        // Reset selected index if out of bounds
        if self.selected_index >= self.filtered_challenges.len() && !self.filtered_challenges.is_empty() {
            self.selected_index = 0;
        }
    }

    /// Get a random challenge from filtered set
    fn get_random_challenge(&self) -> Option<Challenge> {
        if self.filtered_challenges.is_empty() {
            return None;
        }

        // Use proper RNG from rand crate
        let mut rng = rand::thread_rng();
        let challenge_idx = self.filtered_challenges.choose(&mut rng).copied()?;
        Some(self.all_challenges[challenge_idx].clone())
    }

    /// Shows the challenge list and returns the selected challenge
    pub fn show(mut self) -> Result<Option<Challenge>> {
        self.apply_filters();

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        result
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<Option<Challenge>> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                match self.display_mode {
                    DisplayMode::List => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(None);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if self.selected_index < self.filtered_challenges.len().saturating_sub(1) {
                                    self.selected_index += 1;
                                }
                            }
                            KeyCode::Enter => {
                                if !self.filtered_challenges.is_empty() {
                                    let challenge_idx = self.filtered_challenges[self.selected_index];
                                    return Ok(Some(self.all_challenges[challenge_idx].clone()));
                                }
                            }
                            KeyCode::Char('f') => {
                                // Toggle to filter panel
                                self.display_mode = DisplayMode::FilterPanel;
                            }
                            KeyCode::Char('r') => {
                                // Quick practice - random challenge
                                return Ok(self.get_random_challenge());
                            }
                            KeyCode::Char('a') => {
                                // Show all
                                self.filter_mode = FilterMode::All;
                                self.apply_filters();
                            }
                            KeyCode::Char('i') => {
                                // Focus mode - incomplete only
                                self.filter_mode = FilterMode::Incomplete;
                                self.apply_filters();
                            }
                            KeyCode::Char('c') => {
                                // Completed only
                                self.filter_mode = FilterMode::Completed;
                                self.apply_filters();
                            }
                            _ => {}
                        }
                    }
                    DisplayMode::FilterPanel => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                // Return to list
                                self.display_mode = DisplayMode::List;
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if self.filter_panel_selected > 0 {
                                    self.filter_panel_selected -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if self.filter_panel_selected < self.available_tags.len().saturating_sub(1) {
                                    self.filter_panel_selected += 1;
                                }
                            }
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                // Toggle selected tag filter
                                if !self.available_tags.is_empty() && self.filter_panel_selected < self.available_tags.len() {
                                    let tag = self.available_tags[self.filter_panel_selected].clone();
                                    if self.tag_filters.contains(&tag) {
                                        self.tag_filters.remove(&tag);
                                    } else {
                                        self.tag_filters.insert(tag);
                                    }
                                    self.apply_filters();
                                }
                            }
                            KeyCode::Char('x') => {
                                // Clear all tag filters
                                self.tag_filters.clear();
                                self.apply_filters();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn render(&self, f: &mut ratatui::Frame) {
        let size = f.area();

        match self.display_mode {
            DisplayMode::List => self.render_list_view(f, size),
            DisplayMode::FilterPanel => self.render_filter_panel(f, size),
        }
    }

    fn render_list_view(&self, f: &mut ratatui::Frame, area: Rect) {
        // Create main layout
        let chunks = Layout::default()
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(2),  // Filter status
                Constraint::Min(0),     // Challenge list
                Constraint::Length(4),  // Footer
            ])
            .split(area);

        self.render_title(f, chunks[0]);
        self.render_filter_status(f, chunks[1]);
        self.render_list(f, chunks[2]);
        self.render_footer(f, chunks[3]);
    }

    fn render_filter_panel(&self, f: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Min(0),     // Tag list
                Constraint::Length(4),  // Footer
            ])
            .split(area);

        let title = Paragraph::new("TAG FILTERS")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));
        f.render_widget(title, chunks[0]);

        // Render tag list
        let items: Vec<ListItem> = self.available_tags
            .iter()
            .enumerate()
            .map(|(i, tag)| {
                let is_selected = self.filter_panel_selected == i;
                let is_active = self.tag_filters.contains(tag);

                let checkbox = if is_active { "[x]" } else { "[ ]" };
                let prefix = if is_selected { "> " } else { "  " };
                let content = format!("{}{} {}", prefix, checkbox, tag);

                let style = if is_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if is_active {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)));
        f.render_widget(list, chunks[1]);

        let footer_text = vec![
            Line::from("↑/↓: Navigate  Space/Enter: Toggle  x: Clear All"),
            Line::from("Esc: Back to List"),
        ];
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));
        f.render_widget(footer, chunks[2]);
    }

    fn render_title(&self, f: &mut ratatui::Frame, area: Rect) {
        let title = Paragraph::new("CHALLENGE SELECTION")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM));

        f.render_widget(title, area);
    }

    fn render_filter_status(&self, f: &mut ratatui::Frame, area: Rect) {
        let mode_text = match self.filter_mode {
            FilterMode::All => "All",
            FilterMode::Incomplete => "Incomplete",
            FilterMode::Completed => "Completed",
        };

        let tag_text = if self.tag_filters.is_empty() {
            String::new()
        } else {
            let tags: Vec<&str> = self.tag_filters.iter().map(|s| s.as_str()).collect();
            format!(" | Tags: {}", tags.join(", "))
        };

        let status = format!(
            "Showing: {} ({}/{}){}",
            mode_text,
            self.filtered_challenges.len(),
            self.all_challenges.len(),
            tag_text
        );

        let status_widget = Paragraph::new(status)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Left);

        f.render_widget(status_widget, area);
    }

    fn render_list(&self, f: &mut ratatui::Frame, area: Rect) {
        if self.filtered_challenges.is_empty() {
            let empty_msg = Paragraph::new("No challenges match the current filters.\nPress 'a' to show all, or 'f' to adjust filters.")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            f.render_widget(empty_msg, area);
            return;
        }

        let items: Vec<ListItem> = self.filtered_challenges
            .iter()
            .enumerate()
            .map(|(display_idx, &challenge_idx)| {
                let challenge = &self.all_challenges[challenge_idx];

                // Show completion status if we have progress
                let completion_marker = if let Some(ref progress) = self.progress {
                    if let Some(stats) = progress.get_challenge_stats(challenge.id()) {
                        if stats.is_completed() {
                            " ✓"
                        } else {
                            ""
                        }
                    } else {
                        ""
                    }
                } else {
                    ""
                };

                let difficulty_tag = challenge
                    .difficulty()
                    .map(|d| format!(" [{}]", d))
                    .unwrap_or_default();

                let number = format!("{:2}. ", display_idx + 1);
                let title = challenge.title();
                let content = format!("{}{}{}{}", number, title, difficulty_tag, completion_marker);

                let style = if display_idx == self.selected_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if display_idx == self.selected_index { "> " } else { "  " };
                ListItem::new(Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(content, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::NONE));

        f.render_widget(list, area);
    }

    fn render_footer(&self, f: &mut ratatui::Frame, area: Rect) {
        let help_lines = vec![
            Line::from("↑/↓: Navigate  Enter: Select  r: Random  f: Filters"),
            Line::from("a: All  i: Incomplete  c: Completed  q/Esc: Quit"),
        ];

        let footer = Paragraph::new(help_lines)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP));

        f.render_widget(footer, area);
    }
}

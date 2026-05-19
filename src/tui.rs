use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{io, time::Duration};

use crate::config::{AliasConfig, AliasEntry, Config};

pub fn run_tui(config: &Config) -> Result<Option<String>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new(config);
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
        return Ok(None);
    }

    Ok(res?)
}

struct App {
    aliases: Vec<(String, String, bool, Option<String>)>, // (name, display_value, is_parallel, description)
    filtered: Vec<usize>, // indices into aliases matching the current filter
    filter: String,
    search_active: bool,
    state: ListState,
}

impl App {
    fn new(config: &Config) -> App {
        let mut aliases: Vec<(String, String, bool, Option<String>)> = config
            .aliases
            .iter()
            .map(|(k, ac): (&String, &AliasConfig)| {
                let (display, is_parallel) = match &ac.entry {
                    AliasEntry::Single(s) => (s.clone(), false),
                    AliasEntry::Parallel(cmds) => (cmds.join(", "), true),
                };
                (k.clone(), display, is_parallel, ac.description.clone())
            })
            .collect();

        // sort for consistent display
        aliases.sort_by(|a, b| a.0.cmp(&b.0));

        let filtered: Vec<usize> = (0..aliases.len()).collect();
        let mut state = ListState::default();
        if !aliases.is_empty() {
            state.select(Some(0));
        }

        App { aliases, filtered, filter: String::new(), search_active: false, state }
    }

    fn apply_filter(&mut self) {
        let q = self.filter.to_lowercase();
        self.filtered = self
            .aliases
            .iter()
            .enumerate()
            .filter(|(_, (name, _, _, _))| name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        // reset selection so we don't point at a now-invisible row
        if self.filtered.is_empty() {
            self.state.select(None);
        } else {
            self.state.select(Some(0));
        }
    }

    fn next(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.filtered.len() - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 { self.filtered.len() - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<Option<String>> {
    loop {
        terminal
            .draw(|f| ui(f, &mut app))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if app.search_active {
                        match key.code {
                            KeyCode::Esc => {
                                // exit search mode and clear the filter
                                app.search_active = false;
                                app.filter.clear();
                                app.apply_filter();
                            }
                            KeyCode::Backspace => {
                                app.filter.pop();
                                app.apply_filter();
                            }
                            KeyCode::Up => app.previous(),
                            KeyCode::Down => app.next(),
                            KeyCode::Enter => {
                                if let Some(i) = app.state.selected() {
                                    if i < app.filtered.len() {
                                        return Ok(Some(app.aliases[app.filtered[i]].0.clone()));
                                    }
                                }
                            }
                            KeyCode::Char(c) => {
                                app.filter.push(c);
                                app.apply_filter();
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                            KeyCode::Char('/') => {
                                app.search_active = true;
                            }
                            KeyCode::Up | KeyCode::Char('k') => app.previous(),
                            KeyCode::Down | KeyCode::Char('j') => app.next(),
                            KeyCode::Enter => {
                                if let Some(i) = app.state.selected() {
                                    if i < app.filtered.len() {
                                        return Ok(Some(app.aliases[app.filtered[i]].0.clone()));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        // bottom panel is 4 lines: 2 borders + 2 content rows (description + search or hints)
        .constraints([Constraint::Min(0), Constraint::Length(4)].as_ref())
        .split(f.area());

    // inner width minus the highlight symbol so long commands don't get clipped silently
    let available_width = (chunks[0].width as usize).saturating_sub(7);

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&idx| {
            let (name, cmd, is_parallel, _desc) = &app.aliases[idx];
            let prefix = format!("{}  ➜  ", name);
            let reserved = prefix.len() + if *is_parallel { 11 } else { 0 };
            let max_cmd = available_width.saturating_sub(reserved).max(8);
            let truncated = if cmd.len() > max_cmd {
                format!("{}…", &cmd[..max_cmd.saturating_sub(1)])
            } else {
                cmd.clone()
            };

            if *is_parallel {
                let line = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled("[parallel] ", Style::default().fg(Color::Yellow)),
                    Span::styled(truncated, Style::default().fg(Color::Cyan)),
                ]);
                ListItem::new(line)
            } else {
                let line = Line::from(vec![
                    Span::raw(prefix),
                    Span::styled(truncated, Style::default().fg(Color::Cyan)),
                ]);
                ListItem::new(line)
            }
        })
        .collect();

    let list_title = if !app.filter.is_empty() {
        format!(" 🐙 CAWA Aliases ({} matches) ", app.filtered.len())
    } else {
        " 🐙 CAWA Aliases ".to_string()
    };

    let aliases_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(list_title))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol(">> ");

    f.render_stateful_widget(aliases_list, chunks[0], &mut app.state);

    // build the bottom panel: description on top, search input or key hints below
    let desc_line = match app.state.selected() {
        Some(i) if i < app.filtered.len() => {
            let desc = app.aliases[app.filtered[i]].3.as_deref().unwrap_or("");
            Line::from(Span::styled(desc, Style::default().fg(Color::Gray)))
        }
        _ => Line::from(""),
    };

    let hint_line = if app.search_active {
        Line::from(vec![
            Span::styled("/ ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::styled(app.filter.as_str(), Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::Yellow)), // cursor indicator
            Span::styled(
                "   Esc: clear  ↑/↓: navigate  Enter: execute",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else if app.aliases.is_empty() {
        Line::from(Span::styled(
            "No aliases defined. Use `cs add` to create one.",
            Style::default().fg(Color::Gray),
        ))
    } else {
        Line::from(Span::styled(
            "↑/↓: Navigate • Enter: Execute • /: Search • q: Quit",
            Style::default().fg(Color::Gray),
        ))
    };

    let bottom = Paragraph::new(Text::from(vec![desc_line, hint_line]))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(bottom, chunks[1]);
}

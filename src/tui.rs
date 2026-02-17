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
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{io, time::Duration};

use crate::config::{AliasEntry, Config};

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
    aliases: Vec<(String, String)>, // (name, display_value)
    state: ListState,
}

impl App {
    fn new(config: &Config) -> App {
        let mut aliases: Vec<(String, String)> = config
            .aliases
            .iter()
            .map(|(k, v)| {
                let display = match v {
                    AliasEntry::Single(s) => s.clone(),
                    AliasEntry::Parallel(cmds) => format!("[{}]", cmds.join(", ")),
                };
                (k.clone(), display)
            })
            .collect();

        // sort for consistent display
        aliases.sort_by(|a, b| a.0.cmp(&b.0));

        let mut state = ListState::default();
        if !aliases.is_empty() {
            state.select(Some(0));
        }

        App { aliases, state }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.aliases.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.aliases.len() - 1
                } else {
                    i - 1
                }
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
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                        KeyCode::Up | KeyCode::Char('k') => app.previous(),
                        KeyCode::Down | KeyCode::Char('j') => app.next(),
                        KeyCode::Enter => {
                            if let Some(i) = app.state.selected() {
                                if i < app.aliases.len() {
                                    return Ok(Some(app.aliases[i].0.clone()));
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

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    let items: Vec<ListItem> = app
        .aliases
        .iter()
        .map(|(name, cmd)| {
            let line = format!("{}  ➜  {}", name, cmd);
            ListItem::new(line).style(Style::default().fg(Color::White))
        })
        .collect();

    let aliases_list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 🐙 CAWA Aliases "),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(aliases_list, chunks[0], &mut app.state);

    let help_text = match app.state.selected() {
        Some(_) => "↑/↓: Navigate • Enter: Execute • q: Quit",
        None => "No aliases defined. Use `cs add` to create one.",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(help, chunks[1]);
}

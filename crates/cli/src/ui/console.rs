//! The attach console: live server output above an input line, in the shared
//! inline viewport. Detaching never touches the server — stopping a workload
//! is always an explicit act, not a side effect of closing a view.

use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::screen;

const HISTORY: usize = 2000;
const POLL: Duration = Duration::from_millis(100);
const PAGE: usize = 10;

pub enum ConsoleEvent {
    Output(String),
    Reply(String),
    Notice(String),
    /// Ends the console; the message is handed back to the caller.
    Closed(String),
}

enum Kind {
    Output,
    Echo,
    Reply,
    Notice,
}

struct Entry {
    kind: Kind,
    text: String,
}

struct State {
    lines: VecDeque<Entry>,
    input: String,
    /// Lines scrolled up from the live tail; 0 follows new output.
    scroll: usize,
}

impl State {
    fn push(&mut self, kind: Kind, text: String) {
        if self.lines.len() == HISTORY {
            self.lines.pop_front();
        }
        self.lines.push_back(Entry { kind, text });
    }

    fn scroll_up(&mut self, by: usize) {
        self.scroll = (self.scroll + by).min(self.lines.len().saturating_sub(1));
    }
}

/// Run the console until the user detaches (Esc / Ctrl-C / Ctrl-D) or the
/// event stream closes; returns the close message, if any. Blocking — run it
/// off the async runtime.
pub fn run(
    title: &str,
    backfill: Vec<String>,
    mut events: UnboundedReceiver<ConsoleEvent>,
    commands: UnboundedSender<String>,
) -> Result<Option<String>> {
    let mut state = State {
        lines: VecDeque::new(),
        input: String::new(),
        scroll: 0,
    };
    for line in backfill {
        state.push(Kind::Output, line);
    }

    enable_raw_mode()?;
    let result = loop {
        let mut closed = None;
        while let Ok(event) = events.try_recv() {
            match event {
                ConsoleEvent::Output(text) => state.push(Kind::Output, text),
                ConsoleEvent::Reply(text) => state.push(Kind::Reply, text),
                ConsoleEvent::Notice(text) => state.push(Kind::Notice, text),
                ConsoleEvent::Closed(message) => closed = Some(message),
            }
        }
        if let Some(message) = closed {
            break Ok(Some(message));
        }
        let drawn = screen::with_min(screen::MAX_HEIGHT, |terminal| {
            terminal.draw(|frame| draw(frame, title, &state))?;
            Ok(())
        });
        if let Err(e) = drawn {
            break Err(e);
        }
        if !event::poll(POLL)? {
            continue;
        }
        let key = match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => key,
            Ok(_) => continue,
            Err(e) => break Err(e.into()),
        };
        if let Some(outcome) = on_key(key, &mut state, &commands) {
            break outcome;
        }
    };
    let _ = disable_raw_mode();
    screen::blank();
    result
}

type Outcome = Option<Result<Option<String>>>;

fn on_key(key: KeyEvent, state: &mut State, commands: &UnboundedSender<String>) -> Outcome {
    match key.code {
        KeyCode::Esc => return Some(Ok(None)),
        KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(Ok(None))
        }
        KeyCode::Enter => {
            let command = state.input.trim().to_string();
            state.input.clear();
            if !command.is_empty() {
                state.push(Kind::Echo, format!("» {command}"));
                state.scroll = 0;
                if commands.send(command).is_err() {
                    return Some(Ok(Some("console closed".into())));
                }
            }
        }
        KeyCode::Backspace => {
            state.input.pop();
        }
        KeyCode::Up => state.scroll_up(1),
        KeyCode::Down => state.scroll = state.scroll.saturating_sub(1),
        KeyCode::PageUp => state.scroll_up(PAGE),
        KeyCode::PageDown => state.scroll = state.scroll.saturating_sub(PAGE),
        KeyCode::End => state.scroll = 0,
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.input.push(c);
        }
        _ => {}
    }
    None
}

fn draw(frame: &mut Frame, title: &str, state: &State) {
    let block = Block::bordered()
        .border_style(Style::default().fg(Color::DarkGray))
        .title(title.to_string())
        .title_bottom(Line::from(" Enter send · Esc detach · ↑/↓ scroll ").right_aligned());
    let inner = block.inner(frame.area());
    frame.render_widget(block, frame.area());

    let rows = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    let visible = rows[0].height as usize;
    let len = state.lines.len();
    let scroll = state.scroll.min(len.saturating_sub(visible));
    let end = len - scroll;
    let start = end.saturating_sub(visible);
    let lines: Vec<Line> = state
        .lines
        .iter()
        .skip(start)
        .take(end - start)
        .map(entry_line)
        .collect();
    frame.render_widget(Paragraph::new(lines), rows[0]);

    let mut input = vec![
        Span::styled("> ", Style::default().fg(Color::Cyan)),
        Span::raw(state.input.clone()),
        Span::styled("▏", Style::default().fg(Color::Cyan)),
    ];
    if scroll > 0 {
        input.push(Span::styled(
            format!("  (scrolled {scroll}, End to follow)"),
            Style::default().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(input)), rows[1]);
}

fn entry_line(entry: &Entry) -> Line<'_> {
    let text = entry.text.as_str();
    match entry.kind {
        Kind::Output => Line::from(text),
        Kind::Echo => Line::from(Span::styled(
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Kind::Reply => Line::from(Span::styled(text, Style::default().fg(Color::Cyan))),
        Kind::Notice => Line::from(Span::styled(text, Style::default().fg(Color::Yellow))),
    }
}

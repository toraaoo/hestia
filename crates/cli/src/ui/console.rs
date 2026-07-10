//! The attach console: a fullscreen alternate-screen session — live server
//! output above an input line. Fullscreen rather than the shared inline
//! viewport because an inline viewport is laid out once at a fixed height and
//! cannot follow a terminal resize; the alternate screen re-wraps every frame
//! and hands the original terminal back intact on detach. Detaching never
//! touches the server — stopping a workload is always an explicit act, not a
//! side effect of closing a view.

use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use anyhow::Result;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::{Frame, Terminal};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

const HISTORY: usize = 2000;
const POLL: Duration = Duration::from_millis(50);
const WHEEL: usize = 3;
const MIN_WIDTH: u16 = 80;
const MIN_HEIGHT: u16 = 24;

pub enum ConsoleEvent {
    Output(String),
    Reply(String),
    Notice(String),
    /// Ends the console; the message is handed back to the caller.
    Closed(String),
}

#[derive(Clone, Copy)]
enum Kind {
    Output,
    Echo,
    Reply,
    Notice,
}

impl Kind {
    fn style(self) -> Style {
        match self {
            Kind::Output => Style::default(),
            Kind::Echo => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            Kind::Reply => Style::default().fg(Color::Cyan),
            Kind::Notice => Style::default().fg(Color::Yellow),
        }
    }
}

struct Entry {
    kind: Kind,
    text: String,
}

impl Entry {
    fn line(&self) -> Line<'_> {
        Line::styled(self.text.as_str(), self.kind.style())
    }
}

/// The log viewport as measured at the last draw: what scroll clamping and
/// scroll anchoring need between frames.
#[derive(Default, Clone, Copy)]
struct View {
    width: u16,
    height: usize,
    max_scroll: usize,
}

struct State {
    lines: VecDeque<Entry>,
    input: String,
    /// Caret position in `input`, in chars.
    cursor: usize,
    /// Wrapped rows scrolled up from the live tail; 0 follows new output.
    scroll: usize,
    view: View,
}

impl State {
    fn new() -> Self {
        State {
            lines: VecDeque::new(),
            input: String::new(),
            cursor: 0,
            scroll: 0,
            view: View::default(),
        }
    }

    fn push(&mut self, kind: Kind, text: String) {
        if let Some((first, rest)) = text.split_once('\n') {
            self.push(kind, first.trim_end_matches('\r').to_string());
            self.push(kind, rest.to_string());
            return;
        }
        if self.lines.len() == HISTORY {
            self.lines.pop_front();
        }
        let entry = Entry { kind, text };
        // Anchor the reading position: rows appended below the tail while
        // scrolled up must not slide the view towards them.
        if self.scroll > 0 {
            self.scroll += wrapped_rows(&entry, self.view.width);
        }
        self.lines.push_back(entry);
    }

    fn scroll_up(&mut self, by: usize) {
        self.scroll = (self.scroll + by).min(self.view.max_scroll);
    }

    fn scroll_down(&mut self, by: usize) {
        self.scroll = self.scroll.saturating_sub(by);
    }

    fn page(&self) -> usize {
        self.view.height.max(1)
    }

    fn insert(&mut self, c: char) {
        self.input.insert(byte_of(&self.input, self.cursor), c);
        self.cursor += 1;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.input.remove(byte_of(&self.input, self.cursor));
        }
    }

    fn delete(&mut self) {
        if self.cursor < self.input.chars().count() {
            self.input.remove(byte_of(&self.input, self.cursor));
        }
    }

    fn take_input(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.input)
    }
}

fn byte_of(input: &str, cursor: usize) -> usize {
    input
        .char_indices()
        .nth(cursor)
        .map_or(input.len(), |(byte, _)| byte)
}

fn wrapped_rows(entry: &Entry, width: u16) -> usize {
    Paragraph::new(entry.line())
        .wrap(Wrap { trim: false })
        .line_count(width)
        .max(1)
}

/// Run the console until the user detaches (Esc / Ctrl-C / Ctrl-D) or the
/// event stream closes; returns the close message, if any. Blocking — run it
/// off the async runtime. Owns the whole terminal for its lifetime: raw mode
/// plus the alternate screen, restored on exit.
pub fn run(
    title: &str,
    backfill: Vec<String>,
    mut events: UnboundedReceiver<ConsoleEvent>,
    commands: UnboundedSender<String>,
) -> Result<Option<String>> {
    let mut state = State::new();
    for line in backfill {
        state.push(Kind::Output, line);
    }
    enable_raw_mode()?;
    let result = match execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture) {
        Ok(()) => drive(title, &mut state, &mut events, &commands),
        Err(e) => Err(e.into()),
    };
    let _ = execute!(io::stderr(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
    result
}

fn drive(
    title: &str,
    state: &mut State,
    events: &mut UnboundedReceiver<ConsoleEvent>,
    commands: &UnboundedSender<String>,
) -> Result<Option<String>> {
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stderr()))?;
    let mut dirty = true;
    loop {
        while let Ok(event) = events.try_recv() {
            match event {
                ConsoleEvent::Output(text) => state.push(Kind::Output, text),
                ConsoleEvent::Reply(text) => state.push(Kind::Reply, text),
                ConsoleEvent::Notice(text) => state.push(Kind::Notice, text),
                ConsoleEvent::Closed(message) => return Ok(Some(message)),
            }
            dirty = true;
        }
        if dirty {
            terminal.draw(|frame| draw(frame, title, state))?;
            dirty = false;
        }
        if !event::poll(POLL)? {
            continue;
        }
        // Drain every pending event before redrawing, so a paste or held key
        // costs one frame, not one frame per keystroke.
        loop {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if let Some(closed) = on_key(key, state, commands) {
                        return Ok(closed);
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollUp => state.scroll_up(WHEEL),
                    MouseEventKind::ScrollDown => state.scroll_down(WHEEL),
                    _ => {}
                },
                _ => {}
            }
            dirty = true;
            if !event::poll(Duration::ZERO)? {
                break;
            }
        }
    }
}

/// `Some` ends the console: `Some(None)` is a detach, `Some(Some(msg))` a
/// close with a message for the caller.
fn on_key(
    key: KeyEvent,
    state: &mut State,
    commands: &UnboundedSender<String>,
) -> Option<Option<String>> {
    match key.code {
        KeyCode::Esc => return Some(None),
        KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Some(None)
        }
        KeyCode::Enter => {
            let command = state.take_input().trim().to_string();
            if !command.is_empty() {
                state.push(Kind::Echo, format!("» {command}"));
                state.scroll = 0;
                if commands.send(command).is_err() {
                    return Some(Some("console closed".into()));
                }
            }
        }
        KeyCode::Backspace => state.backspace(),
        KeyCode::Delete => state.delete(),
        KeyCode::Left => state.cursor = state.cursor.saturating_sub(1),
        KeyCode::Right => state.cursor = (state.cursor + 1).min(state.input.chars().count()),
        KeyCode::Home => state.cursor = 0,
        KeyCode::End => state.cursor = state.input.chars().count(),
        KeyCode::Up => state.scroll_up(1),
        KeyCode::Down => state.scroll_down(1),
        KeyCode::PageUp => state.scroll_up(state.page()),
        KeyCode::PageDown => state.scroll_down(state.page()),
        KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => state.insert(c),
        _ => {}
    }
    None
}

fn draw(frame: &mut Frame, title: &str, state: &mut State) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);
    draw_header(frame, rows[0], title, state.scroll);
    draw_log(frame, rows[1], state);
    draw_input(frame, rows[2], state);
}

fn draw_too_small(frame: &mut Frame, area: Rect) {
    let rows = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Min(0),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(Line::styled(
            format!("terminal too small — need {MIN_WIDTH}×{MIN_HEIGHT}"),
            Style::default().fg(Color::Yellow),
        ))
        .centered(),
        rows[1],
    );
}

fn draw_header(frame: &mut Frame, area: Rect, title: &str, scroll: usize) {
    let hint = if scroll > 0 {
        format!("scrolled ↑{scroll} · ↓ follows")
    } else {
        "Enter send · Esc detach · ↑/↓ scroll".to_string()
    };
    let hint = Span::styled(hint, Style::default().fg(Color::DarkGray));
    let [title_area, hint_area] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(hint.width() as u16)])
            .areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled(
            title.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        title_area,
    );
    frame.render_widget(Paragraph::new(hint), hint_area);
}

fn draw_log(frame: &mut Frame, area: Rect, state: &mut State) {
    let text = Text::from(state.lines.iter().map(Entry::line).collect::<Vec<_>>());
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let total = paragraph.line_count(area.width);
    let height = area.height as usize;
    let max_scroll = total.saturating_sub(height);
    state.scroll = state.scroll.min(max_scroll);
    state.view = View {
        width: area.width,
        height,
        max_scroll,
    };
    let top = max_scroll - state.scroll;
    frame.render_widget(
        paragraph.scroll((top.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}

fn draw_input(frame: &mut Frame, area: Rect, state: &State) {
    let [prompt_area, input_area] =
        Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled("> ", Style::default().fg(Color::Cyan))),
        prompt_area,
    );
    let caret = Span::raw(&state.input[..byte_of(&state.input, state.cursor)]).width() as u16;
    let overflow = caret.saturating_sub(input_area.width.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(state.input.as_str()).scroll((0, overflow)),
        input_area,
    );
    frame.set_cursor_position(Position::new(input_area.x + caret - overflow, input_area.y));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_width(width: u16) -> State {
        let mut state = State::new();
        state.view.width = width;
        state.view.max_scroll = usize::MAX;
        state
    }

    #[test]
    fn push_splits_multiline_payloads() {
        let mut state = state_with_width(80);
        state.push(Kind::Reply, "a\r\nb\nc".into());
        let texts: Vec<&str> = state.lines.iter().map(|e| e.text.as_str()).collect();
        assert_eq!(texts, ["a", "b", "c"]);
    }

    #[test]
    fn push_caps_history() {
        let mut state = state_with_width(80);
        for i in 0..(HISTORY + 5) {
            state.push(Kind::Output, i.to_string());
        }
        assert_eq!(state.lines.len(), HISTORY);
        assert_eq!(state.lines.front().unwrap().text, "5");
    }

    #[test]
    fn scrolled_view_stays_anchored_as_output_arrives() {
        let mut state = state_with_width(10);
        state.push(Kind::Output, "old".into());
        state.scroll = 3;
        state.push(Kind::Output, "short".into());
        assert_eq!(state.scroll, 4);
        state.push(Kind::Output, "x".repeat(25));
        assert_eq!(state.scroll, 7);
    }

    #[test]
    fn following_tail_ignores_new_output() {
        let mut state = state_with_width(10);
        state.push(Kind::Output, "line".into());
        assert_eq!(state.scroll, 0);
    }

    #[test]
    fn input_edits_at_char_cursor() {
        let mut state = state_with_width(80);
        for c in "héllo".chars() {
            state.insert(c);
        }
        state.cursor = 1;
        state.delete();
        assert_eq!(state.input, "hllo");
        state.insert('a');
        assert_eq!(state.input, "hallo");
        state.backspace();
        assert_eq!(state.input, "hllo");
        assert_eq!(state.cursor, 1);
    }
}

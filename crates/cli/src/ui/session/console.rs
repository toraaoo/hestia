//! The attach console: live server output above an input line, as a session
//! screen. Detaching never touches the server — stopping a workload is always
//! an explicit act, not a side effect of closing a view.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use tokio::sync::mpsc::UnboundedSender;

use super::{log_header, Flow, Screen};
use crate::ui::components::log::{Kind, LogView};
use crate::ui::components::TextInput;

const WHEEL: usize = 3;

pub enum ConsoleEvent {
    Output(String),
    Reply(String),
    Notice(String),
    /// Ends the console; the message is handed back to the caller.
    Closed(String),
}

pub struct ConsoleScreen {
    title: String,
    log: LogView,
    input: TextInput,
    commands: UnboundedSender<String>,
}

impl ConsoleScreen {
    pub fn new(title: &str, backfill: Vec<String>, commands: UnboundedSender<String>) -> Self {
        let mut log = LogView::default();
        for line in backfill {
            log.push(Kind::Output, line);
        }
        ConsoleScreen {
            title: title.to_string(),
            log,
            input: TextInput::default(),
            commands,
        }
    }
}

impl Screen for ConsoleScreen {
    type Event = ConsoleEvent;
    type Outcome = Option<String>;

    fn wants_mouse(&self) -> bool {
        true
    }

    fn draw(&mut self, frame: &mut Frame) {
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());
        log_header(
            frame,
            rows[0],
            &self.title,
            self.log.scroll(),
            "Enter send · Esc detach · ↑/↓ scroll",
        );
        self.log.render(frame, rows[1]);
        draw_input(frame, rows[2], &self.input);
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        match key.code {
            KeyCode::Esc => return Flow::Done(None),
            KeyCode::Char('c' | 'd') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Done(None)
            }
            KeyCode::Enter => {
                let command = self.input.take().trim().to_string();
                if !command.is_empty() {
                    self.log.push(Kind::Echo, format!("» {command}"));
                    self.log.follow();
                    if self.commands.send(command).is_err() {
                        return Flow::Done(Some("console closed".into()));
                    }
                }
            }
            KeyCode::Up => self.log.scroll_up(1),
            KeyCode::Down => self.log.scroll_down(1),
            KeyCode::PageUp => self.log.scroll_up(self.log.page()),
            KeyCode::PageDown => self.log.scroll_down(self.log.page()),
            _ => {
                self.input.on_key(&key);
            }
        }
        Flow::Continue
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> Flow<Self::Outcome> {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.log.scroll_up(WHEEL),
            MouseEventKind::ScrollDown => self.log.scroll_down(WHEEL),
            _ => {}
        }
        Flow::Continue
    }

    fn on_event(&mut self, event: ConsoleEvent) -> Flow<Self::Outcome> {
        match event {
            ConsoleEvent::Output(text) => self.log.push(Kind::Output, text),
            ConsoleEvent::Reply(text) => self.log.push(Kind::Reply, text),
            ConsoleEvent::Notice(text) => self.log.push(Kind::Notice, text),
            ConsoleEvent::Closed(message) => return Flow::Done(Some(message)),
        }
        Flow::Continue
    }
}

fn draw_input(frame: &mut Frame, area: Rect, input: &TextInput) {
    let [prompt_area, input_area] =
        Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).areas(area);
    frame.render_widget(
        Paragraph::new(Span::styled("> ", Style::default().fg(Color::Cyan))),
        prompt_area,
    );
    input.render(frame, input_area);
}

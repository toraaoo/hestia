//! The one-shot prompt screens behind the `ui` facade — select, multi-select,
//! input, confirm — each a small fullscreen session over the shared
//! components. Borderless: the prompt on top, the body below it, a dim key
//! hint on the bottom row.

use std::convert::Infallible;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{is_cancel, Flow, Screen};
use crate::ui::components::{Picker, PickerItem, SelectList, TextInput};

/// Draw the prompt row and hint row, returning the body area between them.
fn chrome(frame: &mut Frame, prompt: &str, hint: &str) -> Rect {
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());
    frame.render_widget(
        Paragraph::new(Line::from(prompt)).style(Style::default().fg(Color::Cyan)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(hint)).style(Style::default().fg(Color::DarkGray)),
        rows[3],
    );
    rows[2]
}

pub struct SelectScreen {
    prompt: String,
    list: SelectList,
}

impl SelectScreen {
    pub fn new(prompt: &str, items: &[String]) -> Self {
        SelectScreen {
            prompt: prompt.to_string(),
            list: SelectList::new(items.to_vec()),
        }
    }
}

impl Screen for SelectScreen {
    type Event = Infallible;
    type Outcome = Option<usize>;

    fn draw(&mut self, frame: &mut Frame) {
        let body = chrome(frame, &self.prompt, "↑/↓ move · enter select · esc cancel");
        self.list.render(frame, body);
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        if is_cancel(&key) {
            return Flow::Done(None);
        }
        if key.code == KeyCode::Enter {
            return Flow::Done(Some(self.list.selected()));
        }
        self.list.on_key(&key);
        Flow::Continue
    }
}

pub struct MultiSelectScreen {
    prompt: String,
    list: SelectList,
}

impl MultiSelectScreen {
    pub fn new(prompt: &str, items: &[String]) -> Self {
        MultiSelectScreen {
            prompt: prompt.to_string(),
            list: SelectList::new(items.to_vec()).with_checkboxes(),
        }
    }
}

impl Screen for MultiSelectScreen {
    type Event = Infallible;
    type Outcome = Option<Vec<usize>>;

    fn draw(&mut self, frame: &mut Frame) {
        let body = chrome(
            frame,
            &self.prompt,
            "↑/↓ move · space toggle · enter confirm · esc cancel",
        );
        self.list.render(frame, body);
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        if is_cancel(&key) {
            return Flow::Done(None);
        }
        if key.code == KeyCode::Enter {
            return Flow::Done(Some(self.list.chosen()));
        }
        self.list.on_key(&key);
        Flow::Continue
    }
}

pub struct InputScreen {
    prompt: String,
    default: String,
    input: TextInput,
}

impl InputScreen {
    pub fn new(prompt: &str, default: &str) -> Self {
        InputScreen {
            prompt: prompt.to_string(),
            default: default.to_string(),
            input: TextInput::default(),
        }
    }
}

impl Screen for InputScreen {
    type Event = Infallible;
    type Outcome = Option<String>;

    fn draw(&mut self, frame: &mut Frame) {
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());
        let label = Span::styled(
            format!("{}: ", self.prompt),
            Style::default().fg(Color::Cyan),
        );
        let [label_area, field_area] =
            Layout::horizontal([Constraint::Length(label.width() as u16), Constraint::Min(0)])
                .areas(rows[0]);
        frame.render_widget(Paragraph::new(label), label_area);
        self.input.render(frame, field_area);
        if self.input.is_empty() && !self.default.is_empty() {
            frame.render_widget(
                Paragraph::new(self.default.as_str()).style(Style::default().fg(Color::DarkGray)),
                field_area,
            );
        }
        frame.render_widget(
            Paragraph::new(Line::from("enter accept · esc cancel"))
                .style(Style::default().fg(Color::DarkGray)),
            rows[2],
        );
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        match key.code {
            KeyCode::Enter => {
                let typed = self.input.take();
                let value = typed.trim();
                Flow::Done(Some(if value.is_empty() {
                    self.default.clone()
                } else {
                    value.to_string()
                }))
            }
            KeyCode::Esc => Flow::Done(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Flow::Done(None),
            _ => {
                self.input.on_key(&key);
                Flow::Continue
            }
        }
    }
}

pub struct PickerScreen {
    prompt: String,
    picker: Picker,
}

impl PickerScreen {
    pub fn new(prompt: &str, items: Vec<PickerItem>) -> Self {
        PickerScreen {
            prompt: prompt.to_string(),
            picker: Picker::new(items),
        }
    }
}

impl Screen for PickerScreen {
    type Event = Infallible;
    type Outcome = Option<usize>;

    fn draw(&mut self, frame: &mut Frame) {
        let rows =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(frame.area());
        self.picker.render(frame, rows[0], &self.prompt);
        frame.render_widget(
            Paragraph::new(Line::from(
                "type to filter · tab all versions · ↑/↓ move · enter select · esc cancel",
            ))
            .style(Style::default().fg(Color::DarkGray)),
            rows[1],
        );
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        match key.code {
            KeyCode::Esc => return Flow::Done(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Done(None)
            }
            KeyCode::Enter => {
                if let Some(index) = self.picker.selected() {
                    return Flow::Done(Some(index));
                }
            }
            _ => {
                self.picker.on_key(&key);
            }
        }
        Flow::Continue
    }
}

pub struct ConfirmScreen {
    prompt: String,
    list: SelectList,
}

impl ConfirmScreen {
    pub fn new(prompt: &str, yes: &str, no: &str) -> Self {
        ConfirmScreen {
            prompt: prompt.to_string(),
            list: SelectList::new(vec![yes.to_string(), no.to_string()]),
        }
    }
}

impl Screen for ConfirmScreen {
    type Event = Infallible;
    type Outcome = Option<bool>;

    fn draw(&mut self, frame: &mut Frame) {
        let body = chrome(frame, &self.prompt, "↑/↓ move · enter select · esc cancel");
        self.list.render(frame, body);
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        if is_cancel(&key) {
            return Flow::Done(None);
        }
        match key.code {
            KeyCode::Char('y') => Flow::Done(Some(true)),
            KeyCode::Char('n') => Flow::Done(Some(false)),
            KeyCode::Enter => Flow::Done(Some(self.list.selected() == 0)),
            _ => {
                self.list.on_key(&key);
                Flow::Continue
            }
        }
    }
}

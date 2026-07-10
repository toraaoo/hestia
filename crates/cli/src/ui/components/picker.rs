//! A searchable picker: a boxed filter input over a tagged list. The pool is
//! split into stable entries (shown by default) and the rest (Tab toggles
//! them in), so a version list leads with releases and can reach snapshots
//! without leaving the widget.

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::TextInput;

pub struct PickerItem {
    /// The row text; the filter matches against it. Selections come back as
    /// indices into the caller's list, so the label need not be unique.
    pub label: String,
    /// A dim annotation after the label (release / snapshot / beta).
    pub tag: String,
    /// Shown without the Tab toggle.
    pub stable: bool,
}

pub struct Picker {
    items: Vec<PickerItem>,
    filter: TextInput,
    show_all: bool,
    visible: Vec<usize>,
    state: ListState,
}

impl Picker {
    pub fn new(items: Vec<PickerItem>) -> Self {
        let show_all = !items.iter().any(|i| i.stable);
        let mut picker = Picker {
            items,
            filter: TextInput::default(),
            show_all,
            visible: Vec::new(),
            state: ListState::default(),
        };
        picker.refresh();
        picker
    }

    /// The original index of the highlighted item.
    pub fn selected(&self) -> Option<usize> {
        self.visible
            .get(self.state.selected().unwrap_or(0))
            .copied()
    }

    /// The pool label for the status corner: what the filter runs over.
    pub fn pool_label(&self) -> &'static str {
        if self.show_all {
            "all"
        } else {
            "releases"
        }
    }

    pub fn toggle_pool(&mut self) {
        self.show_all = !self.show_all;
        self.refresh();
    }

    /// Apply a key: typing edits the filter, Tab toggles the pool, arrows
    /// move. `true` when consumed; Enter/Esc are the caller's.
    pub fn on_key(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Tab => self.toggle_pool(),
            KeyCode::Up => self.step(-1),
            KeyCode::Down => self.step(1),
            _ => {
                if !self.filter.on_key(key) {
                    return false;
                }
                self.refresh();
            }
        }
        true
    }

    fn step(&mut self, delta: isize) {
        if self.visible.is_empty() {
            return;
        }
        let current = self.state.selected().unwrap_or(0) as isize;
        let next = (current + delta).rem_euclid(self.visible.len() as isize);
        self.state.select(Some(next as usize));
    }

    fn refresh(&mut self) {
        let needle = self.filter.text().to_lowercase();
        self.visible = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| self.show_all || item.stable)
            .filter(|(_, item)| needle.is_empty() || item.label.to_lowercase().contains(&needle))
            .map(|(i, _)| i)
            .collect();
        let selected = self
            .state
            .selected()
            .unwrap_or(0)
            .min(self.visible.len().saturating_sub(1));
        self.state
            .select((!self.visible.is_empty()).then_some(selected));
    }

    /// Draw the boxed filter on top and the matching rows below.
    pub fn render(&mut self, frame: &mut Frame, area: Rect, title: &str) {
        let [box_area, list_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);

        let block = Block::bordered()
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title.to_string());
        let inner = block.inner(box_area);
        frame.render_widget(block, box_area);

        let status = format!("{} · {} match", self.pool_label(), self.visible.len());
        let [input_area, status_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(status.len() as u16 + 1),
        ])
        .areas(inner);
        frame.render_widget(
            Paragraph::new(Span::styled(status, Style::default().fg(Color::DarkGray))),
            status_area,
        );
        self.filter.render(frame, input_area);

        let rows: Vec<ListItem> = self
            .visible
            .iter()
            .map(|&i| {
                let item = &self.items[i];
                let mut spans = vec![Span::raw(item.label.clone())];
                if !item.tag.is_empty() {
                    spans.push(Span::styled(
                        format!("  {}", item.tag),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(rows).highlight_symbol("> ").highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, list_area, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn versions() -> Vec<PickerItem> {
        [
            ("1.21.1", "release", true),
            ("24w40a", "snapshot", false),
            ("1.21", "release", true),
            ("1.20.6", "release", true),
        ]
        .into_iter()
        .map(|(id, tag, stable)| PickerItem {
            label: id.to_string(),
            tag: tag.to_string(),
            stable,
        })
        .collect()
    }

    fn labels(picker: &Picker) -> Vec<&str> {
        picker
            .visible
            .iter()
            .map(|&i| picker.items[i].label.as_str())
            .collect()
    }

    #[test]
    fn stable_pool_by_default_and_tab_widens() {
        let mut picker = Picker::new(versions());
        assert_eq!(labels(&picker), ["1.21.1", "1.21", "1.20.6"]);
        picker.toggle_pool();
        assert_eq!(labels(&picker), ["1.21.1", "24w40a", "1.21", "1.20.6"]);
    }

    #[test]
    fn typing_narrows_the_pool() {
        let mut picker = Picker::new(versions());
        for c in "1.21".chars() {
            picker.filter.insert(c);
        }
        picker.refresh();
        assert_eq!(labels(&picker), ["1.21.1", "1.21"]);
        assert_eq!(picker.selected(), Some(0));
    }

    #[test]
    fn all_unstable_pool_shows_everything() {
        let items = vec![PickerItem {
            label: "24w40a".into(),
            tag: "snapshot".into(),
            stable: false,
        }];
        let picker = Picker::new(items);
        assert_eq!(labels(&picker), ["24w40a"]);
    }

    #[test]
    fn selection_survives_a_narrowing_filter() {
        let mut picker = Picker::new(versions());
        picker.step(1);
        picker.step(1);
        assert_eq!(picker.selected(), Some(3));
        for c in "1.21.1".chars() {
            picker.filter.insert(c);
        }
        picker.refresh();
        assert_eq!(picker.selected(), Some(0));
    }
}

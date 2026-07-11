//! A scrollable highlight list: wrap-around stepping, arrow/vim keys, and an
//! optional checkbox mode for multi-selection.

use std::collections::HashSet;

use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::Frame;

pub struct SelectList {
    items: Vec<String>,
    state: ListState,
    checked: Option<HashSet<usize>>,
}

impl SelectList {
    pub fn new(items: Vec<String>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        SelectList {
            items,
            state,
            checked: None,
        }
    }

    pub fn with_checkboxes(mut self) -> Self {
        self.checked = Some(HashSet::new());
        self
    }

    /// Checkbox mode with a set of indices already checked.
    pub fn with_checked(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        let len = self.items.len();
        self.checked = Some(indices.into_iter().filter(|i| *i < len).collect());
        self
    }

    /// Exactly the checked indices, sorted — empty when nothing is checked,
    /// without [`Self::chosen`]'s highlight fallback.
    pub fn checked(&self) -> Vec<usize> {
        let mut checked: Vec<usize> = self.checked.iter().flatten().copied().collect();
        checked.sort_unstable();
        checked
    }

    pub fn selected(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    pub fn step(&mut self, delta: isize) {
        if self.items.is_empty() {
            return;
        }
        let next = (self.selected() as isize + delta).rem_euclid(self.items.len() as isize);
        self.state.select(Some(next as usize));
    }

    pub fn toggle(&mut self) {
        let current = self.selected();
        if let Some(checked) = self.checked.as_mut() {
            if !checked.remove(&current) {
                checked.insert(current);
            }
        }
    }

    /// The checked indices, sorted — or the highlighted row when nothing is
    /// checked (and in plain single-select mode).
    pub fn chosen(&self) -> Vec<usize> {
        match self.checked.as_ref() {
            Some(checked) if !checked.is_empty() => {
                let mut chosen: Vec<usize> = checked.iter().copied().collect();
                chosen.sort_unstable();
                chosen
            }
            _ if self.items.is_empty() => Vec::new(),
            _ => vec![self.selected()],
        }
    }

    /// Apply a navigation/toggle key; `true` when consumed.
    pub fn on_key(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.step(-1),
            KeyCode::Down | KeyCode::Char('j') => self.step(1),
            KeyCode::Char(' ') if self.checked.is_some() => self.toggle(),
            _ => return false,
        }
        true
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let checked = self.checked.as_ref();
        let list = List::new(self.items.iter().enumerate().map(|(i, item)| {
            let text = match checked {
                Some(checked) => {
                    let mark = if checked.contains(&i) { "[x] " } else { "[ ] " };
                    format!("{mark}{item}")
                }
                None => item.clone(),
            };
            ListItem::new(text)
        }))
        .highlight_symbol("> ")
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, area, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(n: usize) -> SelectList {
        SelectList::new((0..n).map(|i| i.to_string()).collect())
    }

    #[test]
    fn step_wraps_both_ways() {
        let mut list = list(3);
        list.step(-1);
        assert_eq!(list.selected(), 2);
        list.step(1);
        assert_eq!(list.selected(), 0);
        list.step(4);
        assert_eq!(list.selected(), 1);
    }

    #[test]
    fn chosen_falls_back_to_the_highlight() {
        let mut plain = list(3);
        plain.step(1);
        assert_eq!(plain.chosen(), vec![1]);

        let mut multi = list(3).with_checkboxes();
        assert_eq!(multi.chosen(), vec![0]);
        multi.toggle();
        multi.step(2);
        multi.toggle();
        assert_eq!(multi.chosen(), vec![0, 2]);
        multi.toggle();
        assert_eq!(multi.chosen(), vec![0]);
    }

    #[test]
    fn empty_list_is_inert() {
        let mut empty = list(0);
        empty.step(1);
        assert_eq!(empty.selected(), 0);
        assert!(empty.chosen().is_empty());
    }
}

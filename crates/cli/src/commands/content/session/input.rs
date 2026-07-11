//! Key handling per mode. Each handler maps a key to a transition on the
//! session's models; the render side reads the result. The overlay handler runs
//! first (below, in `Screen`) so a modal captures keys while it is open.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{ContentSession, Focus, Mode, Overlay, SessionReport};
use crate::ui::components::SelectList;
use crate::ui::session::Flow;

impl ContentSession {
    pub(super) fn on_key_browse(&mut self, key: KeyEvent) -> Flow<Option<SessionReport>> {
        match key.code {
            KeyCode::Esc => return Flow::Done(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Done(None)
            }
            KeyCode::Up => match self.focus {
                Focus::List if self.catalogue.list.selected().unwrap_or(0) == 0 => {
                    self.focus = Focus::Search;
                }
                _ => self.catalogue.step(&self.base, -1),
            },
            KeyCode::Down => match self.focus {
                Focus::Search => {
                    self.focus = Focus::List;
                    self.catalogue.want_detail();
                }
                Focus::List => self.catalogue.step(&self.base, 1),
            },
            KeyCode::PageUp => self.catalogue.scroll(-10),
            KeyCode::PageDown => self.catalogue.scroll(10),
            KeyCode::Enter => match self.focus {
                Focus::Search => {
                    self.focus = Focus::List;
                    self.catalogue.want_detail();
                }
                Focus::List => {
                    if self.target.is_some() {
                        let plain = self
                            .catalogue
                            .highlighted()
                            .map(|hit| self.installed_entries(hit).is_empty())
                            .unwrap_or(false);
                        if !self.cart.has_changes() && plain {
                            self.toggle_chosen();
                        }
                        if self.cart.has_changes() {
                            self.mode = Mode::Review { cursor: 0 };
                        }
                    } else if let Some(hit) = self.catalogue.highlighted().cloned() {
                        self.open_versions(hit);
                    }
                }
            },
            KeyCode::Char(' ') if matches!(self.focus, Focus::List) => {
                if self.target.is_some() {
                    self.toggle_chosen();
                }
            }
            KeyCode::Char('v') if matches!(self.focus, Focus::List) => {
                if let Some(hit) = self.catalogue.highlighted().cloned() {
                    self.open_versions(hit);
                }
            }
            _ => {
                if self.catalogue.search_key(&key) {
                    self.focus = Focus::Search;
                }
            }
        }
        Flow::Continue
    }

    pub(super) fn on_key_review(
        &mut self,
        key: KeyEvent,
        cursor: usize,
    ) -> Flow<Option<SessionReport>> {
        match key.code {
            KeyCode::Esc => self.mode = Mode::Browse,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Done(None)
            }
            KeyCode::Up => {
                self.mode = Mode::Review {
                    cursor: cursor.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                let last = self.cart.rows().saturating_sub(1);
                self.mode = Mode::Review {
                    cursor: (cursor + 1).min(last),
                }
            }
            KeyCode::Char('v') => {
                if let Some(chosen) = self.cart.chosen.get(cursor) {
                    let project = chosen.project.clone();
                    self.open_versions(project);
                }
            }
            KeyCode::Char('w') if self.target.is_some() => self.open_worlds(),
            KeyCode::Char(' ') | KeyCode::Delete => {
                self.cart.drop_row(cursor);
                if !self.cart.has_changes() {
                    self.mode = Mode::Browse;
                } else {
                    let last = self.cart.rows() - 1;
                    self.mode = Mode::Review {
                        cursor: cursor.min(last),
                    };
                }
            }
            KeyCode::Enter => {
                if self.needs_worlds() {
                    self.open_worlds();
                } else {
                    self.install();
                }
            }
            _ => {}
        }
        Flow::Continue
    }

    /// Open the datapack world multi-select over the target's save worlds.
    fn open_worlds(&mut self) {
        if let Some(target) = self.target.as_ref() {
            if !target.worlds.is_empty() {
                self.overlay = Some(Overlay::Worlds(
                    SelectList::new(target.worlds.clone()).with_checkboxes(),
                    target.worlds.clone(),
                ));
            }
        }
    }

    pub(super) fn on_key_overlay(&mut self, key: KeyEvent) {
        let Some(overlay) = self.overlay.take() else {
            return;
        };
        match overlay {
            Overlay::Versions {
                project,
                mut picker,
            } => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    if let Some((picker, versions)) = &picker {
                        if let Some(index) = picker.selected() {
                            if self.target.is_some() {
                                let version = versions[index].clone();
                                self.cart.pin(&project, &version);
                            }
                        }
                    }
                }
                _ => {
                    if let Some((picker, _)) = picker.as_mut() {
                        picker.on_key(&key);
                    }
                    self.overlay = Some(Overlay::Versions { project, picker });
                }
            },
            Overlay::Worlds(mut list, names) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    self.cart.worlds = list.chosen().iter().map(|&i| names[i].clone()).collect();
                }
                _ => {
                    list.on_key(&key);
                    self.overlay = Some(Overlay::Worlds(list, names));
                }
            },
            Overlay::RemoveWorlds {
                project,
                mut list,
                names,
            } => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    let picked: Vec<String> =
                        list.checked().iter().map(|&i| names[i].clone()).collect();
                    if picked.is_empty() {
                        self.cart.unstage_removal(&project);
                    } else if picked.len() < names.len() {
                        self.cart.narrow_removal(&project, picked);
                    }
                }
                _ => {
                    list.on_key(&key);
                    self.overlay = Some(Overlay::RemoveWorlds {
                        project,
                        list,
                        names,
                    });
                }
            },
        }
    }
}

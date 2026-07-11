//! The session's cross-model operations: the decisions that read the installed
//! state (the target), the browse selection (the catalogue), and the staged
//! batch (the cart) together. The two sub-models stay pure; the coupling lives
//! here.

use client::proto::content::{
    ContentAddItem, ContentAddSpec, ContentKind, ContentProject, InstalledContent,
};

use super::cart::StagedRemoval;
use super::driver::{InstallJob, Removal, Request};
use super::{ContentSession, Mode, Overlay, Target};
use crate::commands::content::format::{version_picker, world_name};
use crate::commands::content::EntryKind;
use crate::ui::components::SelectList;

impl ContentSession {
    /// The target's index entries for a project — several for a datapack
    /// installed into more than one world, at most one otherwise. Local imports
    /// carry no project id and cannot be matched to a hit.
    pub(super) fn installed_entries(&self, project: &ContentProject) -> Vec<&InstalledContent> {
        let Some(target) = self.target.as_ref() else {
            return Vec::new();
        };
        target
            .installed
            .iter()
            .filter(|i| {
                !i.project_id.is_empty() && i.project_id == project.id && i.source == project.source
            })
            .collect()
    }

    /// The index entries a staged removal clears — every copy, narrowed to the
    /// staged worlds when the removal was scoped.
    pub(super) fn staged_records(&self, staged: &StagedRemoval) -> Vec<&InstalledContent> {
        let Some(target) = self.target.as_ref() else {
            return Vec::new();
        };
        target
            .installed
            .iter()
            .filter(|i| {
                i.project_id == staged.project_id
                    && (staged.worlds.is_empty()
                        || staged
                            .worlds
                            .iter()
                            .any(|w| world_name(&i.world) == Some(w.as_str())))
            })
            .collect()
    }

    /// The browse-time "already installed" marker: the installed version — or,
    /// for an instance's datapacks (installed per world), the worlds the pack is
    /// in, which is what "installed" precisely means for that kind.
    pub(super) fn installed_label(&self, project: &ContentProject) -> Option<String> {
        let entries = self.installed_entries(project);
        let first = entries.first()?;
        if self.instance_datapacks() {
            let worlds: Vec<&str> = entries
                .iter()
                .filter_map(|i| world_name(&i.world))
                .collect();
            return Some(format!("in {}", worlds.join(", ")));
        }
        Some(first.version_number.clone())
    }

    /// The review-time marker: what this install overwrites. For an instance's
    /// datapacks that is the overlap between the picked target worlds and the
    /// worlds already holding the pack — empty overlap means the install only
    /// adds fresh copies, so there is nothing to flag.
    pub(super) fn review_marker(&self, project: &ContentProject) -> Option<String> {
        let entries = self.installed_entries(project);
        let first = entries.first()?;
        if self.instance_datapacks() {
            let overlap: Vec<&str> = self
                .cart
                .worlds
                .iter()
                .filter_map(|picked| {
                    entries
                        .iter()
                        .find(|i| world_name(&i.world) == Some(picked.as_str()))
                        .and_then(|i| world_name(&i.world))
                })
                .collect();
            if overlap.is_empty() {
                return None;
            }
            return Some(format!("replaces the copy in {}", overlap.join(", ")));
        }
        Some(format!("replaces {}", first.version_number))
    }

    pub(super) fn instance_datapacks(&self) -> bool {
        self.base.kind == ContentKind::DataPack
            && matches!(
                self.target,
                Some(Target {
                    entry: EntryKind::Instance,
                    ..
                })
            )
    }

    pub(super) fn needs_worlds(&self) -> bool {
        self.instance_datapacks() && !self.cart.chosen.is_empty() && self.cart.worlds.is_empty()
    }

    /// Space on a row. A plain row toggles in and out of the batch; an installed
    /// row cycles keep → reinstall → remove → keep. Staging the removal of a
    /// datapack held by several worlds opens the world list, pre-checked, to
    /// narrow which copies go.
    pub(super) fn toggle_chosen(&mut self) {
        let Some(hit) = self.catalogue.highlighted().cloned() else {
            return;
        };
        let installed = !self.installed_entries(&hit).is_empty();
        let chosen_pos = self.cart.chosen_pos(&hit.id);
        if installed {
            if let Some(pos) = chosen_pos {
                self.cart.chosen.remove(pos);
                self.stage_removal(&hit);
            } else if let Some(pos) = self.cart.removal_pos(&hit.id) {
                self.cart.removals.remove(pos);
            } else {
                self.cart.choose_latest(hit);
            }
        } else if let Some(pos) = chosen_pos {
            self.cart.chosen.remove(pos);
        } else {
            self.cart.choose_latest(hit);
        }
    }

    fn stage_removal(&mut self, hit: &ContentProject) {
        let worlds: Vec<String> = self
            .installed_entries(hit)
            .iter()
            .filter_map(|i| world_name(&i.world))
            .map(str::to_string)
            .collect();
        self.cart.stage_removal(hit.id.clone());
        if self.instance_datapacks() && worlds.len() > 1 {
            let count = worlds.len();
            self.overlay = Some(Overlay::RemoveWorlds {
                project: hit.id.clone(),
                list: SelectList::new(worlds.clone()).with_checked(0..count),
                names: worlds,
            });
        }
    }

    pub(super) fn open_versions(&mut self, project: ContentProject) {
        let picker = self
            .catalogue
            .versions_for(&project.id)
            .map(|v| version_picker(v));
        if picker.is_none() {
            self.catalogue.request_versions(&self.base, &project);
        }
        self.overlay = Some(Overlay::Versions {
            project: Box::new(project),
            picker,
        });
    }

    pub(super) fn install(&mut self) {
        let Some((entry, id)) = self.target.as_ref().map(|t| (t.entry, t.id.clone())) else {
            return;
        };
        let items = self
            .cart
            .chosen
            .iter()
            .map(|c| ContentAddItem {
                project: c.project.id.clone(),
                version: c.version_id.clone(),
                ..ContentAddItem::default()
            })
            .collect();
        let removals = self
            .cart
            .removals
            .iter()
            .map(|staged| Removal {
                key: staged.project_id.clone(),
                worlds: staged.worlds.clone(),
                records: self.staged_records(staged).into_iter().cloned().collect(),
            })
            .collect();
        let spec = ContentAddSpec {
            kind: self.base.kind,
            source: self.base.source.clone(),
            items,
            worlds: self.cart.worlds.clone(),
        };
        let _ = self.requests.send(Request::Install(InstallJob {
            entry,
            id,
            spec,
            removals,
        }));
        self.mode = Mode::Installing { progress: None };
    }
}

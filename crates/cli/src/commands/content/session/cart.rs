//! The staged-changes cart: the checked additions and the removals a review
//! will apply, plus the datapack target worlds. A pure model — every mutation
//! here is a plain edit to the batch; the cross-model decisions (what counts as
//! installed, when to open a world picker) live on the session.

use client::proto::content::{ContentProject, ContentVersion};

/// One checked project with its (optional) version pin.
pub(super) struct Chosen {
    pub project: ContentProject,
    pub version_id: String,
    pub version_label: String,
}

/// An installed project staged for removal. `worlds` narrows an instance
/// datapack to some of the save worlds holding it (bare folder names); empty
/// clears every copy.
pub(super) struct StagedRemoval {
    pub project_id: String,
    pub worlds: Vec<String>,
}

#[derive(Default)]
pub(super) struct Cart {
    pub chosen: Vec<Chosen>,
    pub removals: Vec<StagedRemoval>,
    pub worlds: Vec<String>,
}

impl Cart {
    pub(super) fn is_chosen(&self, project_id: &str) -> bool {
        self.chosen.iter().any(|c| c.project.id == project_id)
    }

    pub(super) fn is_removing(&self, project_id: &str) -> bool {
        self.removals.iter().any(|r| r.project_id == project_id)
    }

    pub(super) fn has_changes(&self) -> bool {
        !self.chosen.is_empty() || !self.removals.is_empty()
    }

    pub(super) fn chosen_pos(&self, project_id: &str) -> Option<usize> {
        self.chosen.iter().position(|c| c.project.id == project_id)
    }

    pub(super) fn removal_pos(&self, project_id: &str) -> Option<usize> {
        self.removals
            .iter()
            .position(|r| r.project_id == project_id)
    }

    /// Add a project to the batch at its latest compatible version.
    pub(super) fn choose_latest(&mut self, project: ContentProject) {
        self.chosen.push(Chosen {
            project,
            version_id: String::new(),
            version_label: "latest".to_string(),
        });
    }

    pub(super) fn stage_removal(&mut self, project_id: String) {
        self.removals.push(StagedRemoval {
            project_id,
            worlds: Vec::new(),
        });
    }

    pub(super) fn unstage_removal(&mut self, project_id: &str) {
        self.removals.retain(|r| r.project_id != project_id);
    }

    pub(super) fn narrow_removal(&mut self, project_id: &str, worlds: Vec<String>) {
        if let Some(staged) = self
            .removals
            .iter_mut()
            .find(|r| r.project_id == project_id)
        {
            staged.worlds = worlds;
        }
    }

    /// Pin a version, whatever state the row was in — a removal-staged row flips
    /// back to installing the picked version.
    pub(super) fn pin(&mut self, project: &ContentProject, version: &ContentVersion) {
        self.unstage_removal(&project.id);
        match self.chosen.iter_mut().find(|c| c.project.id == project.id) {
            Some(chosen) => {
                chosen.version_id = version.id.clone();
                chosen.version_label = version.version_number.clone();
            }
            None => self.chosen.push(Chosen {
                project: project.clone(),
                version_id: version.id.clone(),
                version_label: version.version_number.clone(),
            }),
        }
    }

    /// The 0-indexed review row, addressed across the additions then the
    /// removals; drops it and reports whether the batch still has changes.
    pub(super) fn drop_row(&mut self, cursor: usize) {
        if cursor < self.chosen.len() {
            self.chosen.remove(cursor);
        } else if cursor - self.chosen.len() < self.removals.len() {
            self.removals.remove(cursor - self.chosen.len());
        }
    }

    pub(super) fn rows(&self) -> usize {
        self.chosen.len() + self.removals.len()
    }
}

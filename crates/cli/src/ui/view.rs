//! The presentation-agnostic output model. Commands build `View`s and hand them
//! to `ui::show`; the renderer decides how to present them (ratatui on a
//! terminal, plain text when piped). The future TUI consumes the same `View`s.

pub enum View {
    /// A primary line of output.
    Line(String),
    /// A secondary line (dimmed on a terminal): empty-state notes, hints.
    Note(String),
    /// A key/value block, keys aligned.
    Detail(Vec<(String, String)>),
    /// A titled table; long tables page interactively on a terminal.
    Table {
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}

impl View {
    pub fn line(text: impl Into<String>) -> View {
        View::Line(text.into())
    }

    pub fn note(text: impl Into<String>) -> View {
        View::Note(text.into())
    }

    pub fn detail<K: Into<String>, V: Into<String>>(
        rows: impl IntoIterator<Item = (K, V)>,
    ) -> View {
        View::Detail(
            rows.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }

    pub fn table(
        title: impl Into<String>,
        headers: impl IntoIterator<Item = impl Into<String>>,
        rows: Vec<Vec<String>>,
    ) -> View {
        View::Table {
            title: title.into(),
            headers: headers.into_iter().map(Into::into).collect(),
            rows,
        }
    }
}

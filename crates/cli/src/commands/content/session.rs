//! The fullscreen content session: search a source with a live results list
//! and detail pane, check items (with optional per-item version pins), review
//! the batch, and install it as one daemon job — one alternate-screen flow
//! from query to report.
//!
//! The command owns the `Client` and runs an async driver loop; the blocking
//! screen talks to it over two channels. Search/detail/version lookups are
//! plain request/response calls; `Install` runs the one event-driven job and
//! runs last, because the client `Session` has a single event-callback slot.
//! Search replies carry the sequence number of the query that produced them,
//! so a stale reply (the query changed while it was in flight) is dropped.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use anyhow::Result;
use client::proto::content::{
    ContentAddItem, ContentAddSpec, ContentFailure, ContentKind, ContentProject, ContentVersion,
    InstalledContent, ReleaseChannel, SearchQuery, SearchResult, VersionQuery,
};
use client::proto::minecraft::ProvisionProgress;
use client::Client;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::format::{channel_label, compact, kind_plural, side_label};
use super::EntryKind;
use crate::ui::components::working::draw_working;
use crate::ui::components::{Picker, PickerItem, SelectList, TextInput};
use crate::ui::session::{self, Flow, Screen};

const DEBOUNCE: Duration = Duration::from_millis(250);
const PAGE: u32 = super::PAGE;

/// The entry a session installs into; `None` browses read-only.
pub struct Target {
    pub entry: EntryKind,
    pub id: String,
    pub name: String,
    /// An instance's save worlds, for the datapack world picker.
    pub worlds: Vec<String>,
}

/// What the session resolved to; `None` when the user quit without installing.
pub struct SessionReport {
    pub items: Vec<InstalledContent>,
    pub failures: Vec<ContentFailure>,
    pub error: Option<String>,
}

enum Request {
    Search { seq: u64, query: SearchQuery },
    Detail { source: String, project: String },
    Versions { query: VersionQuery },
    Install { spec: InstallSpec },
}

enum AppEvent {
    Search {
        seq: u64,
        offset: u32,
        result: SearchResult,
    },
    Detail(Box<ContentProject>),
    Versions {
        project: String,
        versions: Vec<ContentVersion>,
    },
    Progress(ProvisionProgress),
    Done {
        items: Vec<InstalledContent>,
        failures: Vec<ContentFailure>,
    },
    Failed {
        message: String,
    },
}

/// Run the content session over `base` (the seeded filters — kind, source,
/// loader, game version, sort). With a target, checked items install into it;
/// without one this is a pure browser.
pub async fn run(
    client: &Client,
    base: SearchQuery,
    target: Option<Target>,
) -> Result<Option<SessionReport>> {
    let (request_tx, request_rx) = unbounded_channel();
    let (event_tx, event_rx) = unbounded_channel();
    let screen = ContentSession::new(base, target, request_tx);
    let driver = drive(client, request_rx, event_tx);
    let (outcome, ()) = tokio::join!(session::run_async(screen, Some(event_rx)), driver);
    outcome
}

/// The async half: answer screen requests over the caller's client. Ends when
/// the screen (the only request sender) is dropped.
async fn drive(
    client: &Client,
    mut requests: UnboundedReceiver<Request>,
    events: UnboundedSender<AppEvent>,
) {
    while let Some(request) = requests.recv().await {
        let event = match request {
            Request::Search { seq, query } => match client.content().search(&query).await {
                Ok(result) => AppEvent::Search {
                    seq,
                    offset: query.offset,
                    result,
                },
                Err(e) => AppEvent::Failed {
                    message: e.to_string(),
                },
            },
            Request::Detail { source, project } => {
                match client.content().project(&source, &project).await {
                    Ok(detail) => AppEvent::Detail(Box::new(detail)),
                    Err(_) => continue,
                }
            }
            Request::Versions { query } => {
                let project = query.project.clone();
                match client.content().versions(&query).await {
                    Ok(versions) => AppEvent::Versions { project, versions },
                    Err(e) => AppEvent::Failed {
                        message: e.to_string(),
                    },
                }
            }
            Request::Install { spec } => {
                let InstallSpec { target, spec } = spec;
                let progress = events.clone();
                let result = match target {
                    InstallTarget::Server(id) => {
                        client
                            .server()
                            .content_add(&id, spec, move |p| {
                                let _ = progress.send(AppEvent::Progress(p.clone()));
                            })
                            .await
                    }
                    InstallTarget::Instance(id) => {
                        client
                            .instance()
                            .content_add(&id, spec, move |p| {
                                let _ = progress.send(AppEvent::Progress(p.clone()));
                            })
                            .await
                    }
                };
                match result {
                    Ok((items, failures)) => AppEvent::Done { items, failures },
                    Err(e) => AppEvent::Failed {
                        message: e.to_string(),
                    },
                }
            }
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

enum InstallTarget {
    Server(String),
    Instance(String),
}

struct InstallSpec {
    target: InstallTarget,
    spec: ContentAddSpec,
}

/// One checked project with its (optional) version pin.
struct Chosen {
    project: ContentProject,
    version_id: String,
    version_label: String,
}

enum Focus {
    Search,
    List,
}

enum Mode {
    Browse,
    Review { cursor: usize },
    Installing { progress: Option<ProvisionProgress> },
    Report(SessionReport),
}

enum Overlay {
    Versions {
        project: Box<ContentProject>,
        picker: Option<(Picker, Vec<ContentVersion>)>,
    },
    Worlds(SelectList, Vec<String>),
}

struct ContentSession {
    base: SearchQuery,
    target: Option<Target>,
    requests: UnboundedSender<Request>,

    mode: Mode,
    overlay: Option<Overlay>,
    focus: Focus,

    search: TextInput,
    debounce: Option<Instant>,
    sent_seq: u64,
    applied_seq: u64,

    hits: Vec<ContentProject>,
    total: u32,
    list: ListState,
    chosen: Vec<Chosen>,
    worlds: Vec<String>,

    details: HashMap<String, ContentProject>,
    detail_requested: HashSet<String>,
    versions: HashMap<String, Vec<ContentVersion>>,
    status: Option<String>,

    /// The detail pane's scroll offset (reset when the highlight moves) and
    /// its geometry as of the last draw — what key clamping and mouse-wheel
    /// hit-testing need between frames.
    detail_scroll: u16,
    detail_max_scroll: u16,
    detail_area: Rect,
}

impl ContentSession {
    fn new(base: SearchQuery, target: Option<Target>, requests: UnboundedSender<Request>) -> Self {
        let mut session = ContentSession {
            search: TextInput::with_text(&base.query),
            base,
            target,
            requests,
            mode: Mode::Browse,
            overlay: None,
            focus: Focus::Search,
            debounce: None,
            sent_seq: 0,
            applied_seq: 0,
            hits: Vec::new(),
            total: 0,
            list: ListState::default(),
            chosen: Vec::new(),
            worlds: Vec::new(),
            details: HashMap::new(),
            detail_requested: HashSet::new(),
            versions: HashMap::new(),
            status: None,
            detail_scroll: 0,
            detail_max_scroll: 0,
            detail_area: Rect::default(),
        };
        session.send_search(0);
        session
    }

    fn query(&self, offset: u32) -> SearchQuery {
        SearchQuery {
            query: self.search.text().trim().to_string(),
            offset,
            limit: PAGE,
            ..self.base.clone()
        }
    }

    fn send_search(&mut self, offset: u32) {
        self.sent_seq += 1;
        let _ = self.requests.send(Request::Search {
            seq: self.sent_seq,
            query: self.query(offset),
        });
    }

    fn searching(&self) -> bool {
        self.applied_seq < self.sent_seq
    }

    fn highlighted(&self) -> Option<&ContentProject> {
        self.hits.get(self.list.selected().unwrap_or(0))
    }

    /// Ask for the highlighted project's long description once.
    fn want_detail(&mut self) {
        let Some((id, source, slug)) = self
            .highlighted()
            .map(|hit| (hit.id.clone(), hit.source.clone(), hit.slug.clone()))
        else {
            return;
        };
        if self.details.contains_key(&id) || !self.detail_requested.insert(id) {
            return;
        }
        let _ = self.requests.send(Request::Detail {
            source,
            project: slug,
        });
    }

    fn version_query(&self, project: &ContentProject) -> VersionQuery {
        VersionQuery {
            source: project.source.clone(),
            project: project.id.clone(),
            loader: self.base.loader.clone(),
            game_version: self.base.game_version.clone(),
        }
    }

    fn open_versions(&mut self, project: ContentProject) {
        let picker = self.versions.get(&project.id).map(|v| version_picker(v));
        if picker.is_none() {
            let _ = self.requests.send(Request::Versions {
                query: self.version_query(&project),
            });
        }
        self.overlay = Some(Overlay::Versions {
            project: Box::new(project),
            picker,
        });
    }

    fn is_chosen(&self, project_id: &str) -> bool {
        self.chosen.iter().any(|c| c.project.id == project_id)
    }

    fn toggle_chosen(&mut self) {
        let Some(hit) = self.highlighted().cloned() else {
            return;
        };
        if let Some(pos) = self.chosen.iter().position(|c| c.project.id == hit.id) {
            self.chosen.remove(pos);
        } else {
            self.chosen.push(Chosen {
                project: hit,
                version_id: String::new(),
                version_label: "latest".to_string(),
            });
        }
    }

    fn pin_version(&mut self, project: &ContentProject, version: &ContentVersion) {
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

    fn step_list(&mut self, delta: isize) {
        if self.hits.is_empty() {
            return;
        }
        let last = self.hits.len() as isize - 1;
        let current = self.list.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, last) as usize;
        self.list.select(Some(next));
        self.detail_scroll = 0;
        self.want_detail();
        if next + 3 >= self.hits.len() && (self.hits.len() as u32) < self.total && !self.searching()
        {
            let offset = self.hits.len() as u32;
            self.send_search(offset);
        }
    }

    fn scroll_detail(&mut self, delta: i32) {
        self.detail_scroll = self
            .detail_scroll
            .saturating_add_signed(delta as i16)
            .min(self.detail_max_scroll);
    }

    fn needs_worlds(&self) -> bool {
        self.base.kind == ContentKind::DataPack
            && matches!(
                self.target,
                Some(Target {
                    entry: EntryKind::Instance,
                    ..
                })
            )
            && self.worlds.is_empty()
    }

    fn install(&mut self) {
        let Some(target) = self.target.as_ref() else {
            return;
        };
        let items = self
            .chosen
            .iter()
            .map(|c| ContentAddItem {
                project: c.project.id.clone(),
                version: c.version_id.clone(),
                ..ContentAddItem::default()
            })
            .collect();
        let spec = ContentAddSpec {
            kind: self.base.kind,
            source: self.base.source.clone(),
            items,
            worlds: self.worlds.clone(),
        };
        let install_target = match target.entry {
            EntryKind::Server => InstallTarget::Server(target.id.clone()),
            EntryKind::Instance => InstallTarget::Instance(target.id.clone()),
        };
        let _ = self.requests.send(Request::Install {
            spec: InstallSpec {
                target: install_target,
                spec,
            },
        });
        self.mode = Mode::Installing { progress: None };
    }

    fn on_key_browse(&mut self, key: KeyEvent) -> Flow<Option<SessionReport>> {
        match key.code {
            KeyCode::Esc => return Flow::Done(None),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Flow::Done(None)
            }
            KeyCode::Up => match self.focus {
                Focus::List if self.list.selected().unwrap_or(0) == 0 => {
                    self.focus = Focus::Search;
                }
                _ => self.step_list(-1),
            },
            KeyCode::Down => match self.focus {
                Focus::Search => {
                    self.focus = Focus::List;
                    self.want_detail();
                }
                Focus::List => self.step_list(1),
            },
            KeyCode::PageUp => self.scroll_detail(-10),
            KeyCode::PageDown => self.scroll_detail(10),
            KeyCode::Enter => match self.focus {
                Focus::Search => {
                    self.focus = Focus::List;
                    self.want_detail();
                }
                Focus::List => {
                    if self.target.is_some() {
                        if self.chosen.is_empty() {
                            self.toggle_chosen();
                        }
                        if !self.chosen.is_empty() {
                            self.mode = Mode::Review { cursor: 0 };
                        }
                    } else if let Some(hit) = self.highlighted().cloned() {
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
                if let Some(hit) = self.highlighted().cloned() {
                    self.open_versions(hit);
                }
            }
            _ => {
                if self.search.on_key(&key) {
                    self.focus = Focus::Search;
                    self.debounce = Some(Instant::now());
                }
            }
        }
        Flow::Continue
    }

    fn on_key_review(&mut self, key: KeyEvent, cursor: usize) -> Flow<Option<SessionReport>> {
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
                self.mode = Mode::Review {
                    cursor: (cursor + 1).min(self.chosen.len().saturating_sub(1)),
                }
            }
            KeyCode::Char('v') => {
                if let Some(chosen) = self.chosen.get(cursor) {
                    let project = chosen.project.clone();
                    self.open_versions(project);
                }
            }
            KeyCode::Char('w') if self.target.is_some() => {
                if let Some(target) = self.target.as_ref() {
                    if !target.worlds.is_empty() {
                        self.overlay = Some(Overlay::Worlds(
                            SelectList::new(target.worlds.clone()).with_checkboxes(),
                            target.worlds.clone(),
                        ));
                    }
                }
            }
            KeyCode::Char(' ') | KeyCode::Delete => {
                if cursor < self.chosen.len() {
                    self.chosen.remove(cursor);
                }
                if self.chosen.is_empty() {
                    self.mode = Mode::Browse;
                } else {
                    self.mode = Mode::Review {
                        cursor: cursor.min(self.chosen.len() - 1),
                    };
                }
            }
            KeyCode::Enter => {
                if self.needs_worlds() {
                    if let Some(target) = self.target.as_ref() {
                        self.overlay = Some(Overlay::Worlds(
                            SelectList::new(target.worlds.clone()).with_checkboxes(),
                            target.worlds.clone(),
                        ));
                    }
                } else {
                    self.install();
                }
            }
            _ => {}
        }
        Flow::Continue
    }

    fn on_key_overlay(&mut self, key: KeyEvent) {
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
                                self.pin_version(&project, &version);
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
                    self.worlds = list.chosen().iter().map(|&i| names[i].clone()).collect();
                }
                _ => {
                    list.on_key(&key);
                    self.overlay = Some(Overlay::Worlds(list, names));
                }
            },
        }
    }
}

impl Screen for ContentSession {
    type Event = AppEvent;
    type Outcome = Option<SessionReport>;

    fn wants_mouse(&self) -> bool {
        true
    }

    fn tick(&mut self) -> bool {
        if let Some(since) = self.debounce {
            if since.elapsed() >= DEBOUNCE {
                self.debounce = None;
                self.send_search(0);
                return true;
            }
        }
        false
    }

    fn on_event(&mut self, event: AppEvent) -> Flow<Self::Outcome> {
        match event {
            AppEvent::Search {
                seq,
                offset,
                result,
            } => {
                if seq < self.sent_seq {
                    return Flow::Continue;
                }
                self.applied_seq = seq;
                self.total = result.total;
                if offset == 0 {
                    self.hits = result.hits;
                    self.list.select((!self.hits.is_empty()).then_some(0));
                } else {
                    let known: HashSet<String> = self.hits.iter().map(|h| h.id.clone()).collect();
                    self.hits
                        .extend(result.hits.into_iter().filter(|h| !known.contains(&h.id)));
                }
                self.status = None;
                self.want_detail();
            }
            AppEvent::Detail(project) => {
                self.details.insert(project.id.clone(), *project);
            }
            AppEvent::Versions { project, versions } => {
                self.versions.insert(project.clone(), versions.clone());
                if let Some(Overlay::Versions {
                    project: wanted,
                    picker,
                }) = self.overlay.as_mut()
                {
                    if wanted.id == project && picker.is_none() {
                        *picker = Some(version_picker(&versions));
                    }
                }
            }
            AppEvent::Progress(progress) => {
                if let Mode::Installing { progress: current } = &mut self.mode {
                    *current = Some(progress);
                }
            }
            AppEvent::Done { items, failures } => {
                self.mode = Mode::Report(SessionReport {
                    items,
                    failures,
                    error: None,
                });
            }
            AppEvent::Failed { message } => match self.mode {
                Mode::Installing { .. } => {
                    self.mode = Mode::Report(SessionReport {
                        items: Vec::new(),
                        failures: Vec::new(),
                        error: Some(message),
                    });
                }
                _ => self.status = Some(message),
            },
        }
        Flow::Continue
    }

    fn on_key(&mut self, key: KeyEvent) -> Flow<Self::Outcome> {
        if self.overlay.is_some() {
            self.on_key_overlay(key);
            return Flow::Continue;
        }
        match &self.mode {
            Mode::Browse => self.on_key_browse(key),
            Mode::Review { cursor } => {
                let cursor = *cursor;
                self.on_key_review(key, cursor)
            }
            Mode::Installing { .. } => Flow::Continue,
            Mode::Report(_) => {
                let Mode::Report(report) = std::mem::replace(&mut self.mode, Mode::Browse) else {
                    unreachable!()
                };
                Flow::Done(Some(report))
            }
        }
    }

    fn on_mouse(&mut self, mouse: MouseEvent) -> Flow<Self::Outcome> {
        if matches!(self.mode, Mode::Browse) && self.overlay.is_none() {
            let over_detail = self
                .detail_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row));
            match (mouse.kind, over_detail) {
                (MouseEventKind::ScrollUp, true) => self.scroll_detail(-3),
                (MouseEventKind::ScrollDown, true) => self.scroll_detail(3),
                (MouseEventKind::ScrollUp, false) => self.step_list(-3),
                (MouseEventKind::ScrollDown, false) => self.step_list(3),
                _ => {}
            }
        }
        Flow::Continue
    }

    fn draw(&mut self, frame: &mut Frame) {
        match &self.mode {
            Mode::Browse => self.draw_browse(frame),
            Mode::Review { cursor } => {
                let cursor = *cursor;
                self.draw_review(frame, cursor)
            }
            Mode::Installing { progress } => draw_working(frame, "installing", progress.as_ref()),
            Mode::Report(report) => draw_report(frame, report),
        }
        if self.overlay.is_some() {
            self.draw_overlay(frame);
        }
    }
}

impl ContentSession {
    fn status_line(&self) -> String {
        if let Some(error) = &self.status {
            return error.clone();
        }
        let mut parts = Vec::new();
        if let Some(loader) = &self.base.loader {
            parts.push(loader.clone());
        }
        if let Some(game) = &self.base.game_version {
            parts.push(game.clone());
        }
        if self.searching() {
            parts.push("searching…".to_string());
        } else {
            parts.push(format!("{} results", self.total));
        }
        parts.join(" · ")
    }

    fn draw_browse(&mut self, frame: &mut Frame) {
        let [box_area, body, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        let title = format!("search {}", kind_plural(self.base.kind));
        let block = Block::bordered()
            .border_style(Style::default().fg(Color::DarkGray))
            .title(title);
        let inner = block.inner(box_area);
        frame.render_widget(block, box_area);
        let status = self.status_line();
        let [input_area, status_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(status.chars().count() as u16 + 1),
        ])
        .areas(inner);
        frame.render_widget(
            Paragraph::new(Span::styled(status, Style::default().fg(Color::DarkGray))),
            status_area,
        );
        self.search
            .render_focused(frame, input_area, matches!(self.focus, Focus::Search));

        let [list_area, detail_area] =
            Layout::horizontal([Constraint::Percentage(55), Constraint::Min(0)]).areas(body);
        self.draw_hits(frame, list_area);
        self.draw_detail(frame, detail_area);

        let hint = match (self.target.is_some(), &self.focus) {
            (_, Focus::Search) => "type to search · ↓ results · esc quit",
            (true, Focus::List) => {
                "↑/↓ move · space select · v version · pgup/pgdn description · enter review · esc quit"
            }
            (false, Focus::List) => "↑/↓ move · enter versions · pgup/pgdn description · esc quit",
        };
        frame.render_widget(
            Paragraph::new(Line::from(hint)).style(Style::default().fg(Color::DarkGray)),
            footer,
        );
    }

    fn draw_hits(&mut self, frame: &mut Frame, area: Rect) {
        if self.hits.is_empty() {
            let text = if self.searching() {
                "searching…"
            } else {
                "no results"
            };
            frame.render_widget(
                Paragraph::new(Line::styled(text, Style::default().fg(Color::DarkGray))),
                area,
            );
            return;
        }
        let with_boxes = self.target.is_some();
        let rows: Vec<ListItem> = self
            .hits
            .iter()
            .map(|hit| {
                let mut spans = Vec::new();
                if with_boxes {
                    let mark = if self.is_chosen(&hit.id) {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    spans.push(Span::raw(mark));
                }
                spans.push(Span::raw(hit.title.clone()));
                spans.push(Span::styled(
                    format!("  ↓{}", compact(hit.downloads)),
                    Style::default().fg(Color::DarkGray),
                ));
                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(rows).highlight_symbol("> ").highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, area, &mut self.list);
    }

    fn draw_detail(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(area).inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 0,
        });
        frame.render_widget(block, area);
        self.detail_area = inner;
        let Some(hit) = self.highlighted() else {
            return;
        };
        let project = self.details.get(&hit.id).unwrap_or(hit);

        let dim = Style::default().fg(Color::DarkGray);
        let mut lines = vec![
            Line::styled(
                project.title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                format!(
                    "by {} · {} downloads",
                    project.author,
                    compact(project.downloads)
                ),
                dim,
            ),
            Line::styled(project.categories.join(" · "), dim),
            Line::styled(
                format!(
                    "client {} · server {}",
                    side_label(project.client_side),
                    side_label(project.server_side)
                ),
                dim,
            ),
            Line::raw(""),
        ];
        for line in project.description.split('\n') {
            lines.push(Line::raw(line.to_string()));
        }
        if !project.body.is_empty() {
            lines.push(Line::raw(""));
            let body = tui_markdown::from_str(&project.body);
            lines.extend(body.lines.into_iter().map(line_to_static));
        }

        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let total = paragraph.line_count(inner.width) as u16;
        self.detail_max_scroll = total.saturating_sub(inner.height);
        self.detail_scroll = self.detail_scroll.min(self.detail_max_scroll);
        frame.render_widget(paragraph.scroll((self.detail_scroll, 0)), inner);
        if self.detail_max_scroll > 0 {
            let position = format!(" {}/{} ", self.detail_scroll, self.detail_max_scroll);
            let corner = Rect {
                x: inner.right().saturating_sub(position.len() as u16),
                y: inner.y,
                width: (position.len() as u16).min(inner.width),
                height: 1,
            };
            frame.render_widget(Paragraph::new(Line::styled(position, dim)), corner);
        }
    }

    fn draw_review(&self, frame: &mut Frame, cursor: usize) {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        let name = self
            .target
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_default();
        frame.render_widget(
            Paragraph::new(Line::styled(
                format!(
                    "review · install {} {} into '{name}'",
                    self.chosen.len(),
                    kind_plural(self.base.kind)
                ),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            header,
        );

        let mut rows: Vec<ListItem> = self
            .chosen
            .iter()
            .map(|c| {
                ListItem::new(Line::from(vec![
                    Span::raw(c.project.title.clone()),
                    Span::styled(
                        format!("  {}", c.version_label),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            })
            .collect();
        if self.base.kind == ContentKind::DataPack && self.target.is_some() {
            let worlds = if self.worlds.is_empty() {
                match self.needs_worlds() {
                    true => "worlds: (none picked — w to pick)".to_string(),
                    false => "world: the server's own".to_string(),
                }
            } else {
                format!("worlds: {}", self.worlds.join(", "))
            };
            rows.push(ListItem::new(Line::styled(
                worlds,
                Style::default().fg(Color::Yellow),
            )));
        }
        let mut state = ListState::default();
        state.select(Some(cursor));
        let list = List::new(rows).highlight_symbol("> ").highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_stateful_widget(list, body, &mut state);

        let hint = if self.base.kind == ContentKind::DataPack {
            "enter install · v version · w worlds · space remove · esc back"
        } else {
            "enter install · v version · space remove · esc back"
        };
        frame.render_widget(
            Paragraph::new(Line::from(hint)).style(Style::default().fg(Color::DarkGray)),
            footer,
        );
    }

    fn draw_overlay(&mut self, frame: &mut Frame) {
        let area = centered(frame.area(), 60, 70);
        frame.render_widget(Clear, area);
        match self.overlay.as_mut() {
            Some(Overlay::Versions { project, picker }) => match picker {
                Some((picker, _)) => {
                    let [picker_area, hint] =
                        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);
                    picker.render(frame, picker_area, &project.title);
                    frame.render_widget(
                        Paragraph::new(Line::from("enter pin version · esc keep latest"))
                            .style(Style::default().fg(Color::DarkGray)),
                        hint,
                    );
                }
                None => {
                    let block = Block::bordered()
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(project.title.clone());
                    let inner = block.inner(area);
                    frame.render_widget(block, area);
                    frame.render_widget(
                        Paragraph::new(Line::styled(
                            "fetching versions…",
                            Style::default().fg(Color::DarkGray),
                        )),
                        inner,
                    );
                }
            },
            Some(Overlay::Worlds(list, _)) => {
                let block = Block::bordered()
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("install into world(s)");
                let inner = block.inner(area);
                frame.render_widget(block, area);
                let [list_area, hint] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
                list.render(frame, list_area);
                frame.render_widget(
                    Paragraph::new(Line::from("space toggle · enter confirm · esc cancel"))
                        .style(Style::default().fg(Color::DarkGray)),
                    hint,
                );
            }
            None => {}
        }
    }
}

fn draw_report(frame: &mut Frame, report: &SessionReport) {
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(
        Paragraph::new(Line::styled(
            "install report",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        header,
    );
    let mut lines = Vec::new();
    for item in &report.items {
        lines.push(Line::raw(format!(
            "installed {} {}",
            item.title, item.version_number
        )));
    }
    for failure in &report.failures {
        let label = if failure.title.is_empty() {
            &failure.item
        } else {
            &failure.title
        };
        lines.push(Line::styled(
            format!("failed {label}: {}", failure.message),
            Style::default().fg(Color::Yellow),
        ));
    }
    if let Some(error) = &report.error {
        lines.push(Line::styled(
            format!("install failed: {error}"),
            Style::default().fg(Color::Red),
        ));
    }
    if lines.is_empty() {
        lines.push(Line::styled(
            "nothing installed",
            Style::default().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
    frame.render_widget(
        Paragraph::new(Line::from("press any key to close"))
            .style(Style::default().fg(Color::DarkGray)),
        footer,
    );
}

fn version_picker(versions: &[ContentVersion]) -> (Picker, Vec<ContentVersion>) {
    let items: Vec<PickerItem> = versions
        .iter()
        .map(|v| PickerItem {
            label: v.version_number.clone(),
            tag: format!(
                "{} · {}",
                channel_label(v.channel),
                v.game_versions.join(", ")
            ),
            stable: v.channel == ReleaseChannel::Release,
        })
        .collect();
    (Picker::new(items), versions.to_vec())
}

/// Detach a rendered markdown line from the source text it borrows, so the
/// detail pane can hold it while the session mutates its own state.
fn line_to_static(line: Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect();
    Line::from(spans)
        .style(line.style)
        .alignment(line.alignment.unwrap_or(ratatui::layout::Alignment::Left))
}

fn centered(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let [_, mid, _] = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .areas(area);
    let [_, mid, _] = Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .areas(mid);
    mid
}

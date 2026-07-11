//! Markdown rendering for session panes: sanitize the source, render it
//! through `tui-markdown`, and hand back owned lines.
//!
//! Project descriptions in the wild mix HTML into their markdown (centered
//! headers, `<img>` badges, `<details>` blocks). The renderer drops HTML
//! events wholesale, which would silently swallow the text inside those tags
//! — so tags are stripped here first, keeping their inner text. Code fences
//! and inline code are left untouched: an angle bracket in a code sample is
//! code, not markup.

use ratatui::text::{Line, Span, Text};

/// Render markdown into owned lines, HTML markup stripped down to its text.
pub fn render(markdown: &str) -> Vec<Line<'static>> {
    let sanitized = sanitize(markdown);
    let text: Text = tui_markdown::from_str(&sanitized);
    text.lines.into_iter().map(own_line).collect()
}

fn own_line(line: Line<'_>) -> Line<'static> {
    let spans: Vec<Span<'static>> = line
        .spans
        .into_iter()
        .map(|s| Span::styled(s.content.into_owned(), s.style))
        .collect();
    Line::from(spans)
        .style(line.style)
        .alignment(line.alignment.unwrap_or(ratatui::layout::Alignment::Left))
}

/// Strip HTML tags (keeping their inner text, `<br>` as a line break) and
/// decode the basic entities, outside of code fences and inline code spans.
fn sanitize(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_fence = false;
    for line in markdown.split_inclusive('\n') {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            in_fence = !in_fence;
            out.push_str(line);
            continue;
        }
        if in_fence {
            out.push_str(line);
        } else {
            sanitize_line(line, &mut out);
        }
    }
    out
}

fn sanitize_line(line: &str, out: &mut String) {
    let mut rest = line;
    let mut in_code = false;
    while let Some(next) = rest.find(|c| c == '`' || (!in_code && (c == '<' || c == '&'))) {
        out.push_str(&rest[..next]);
        rest = &rest[next..];
        let c = rest.chars().next().expect("found char");
        if c == '`' {
            in_code = !in_code;
            out.push('`');
            rest = &rest[1..];
        } else if c == '<' {
            match tag_end(rest) {
                Some((end, is_break)) => {
                    if is_break {
                        out.push('\n');
                    }
                    rest = &rest[end..];
                }
                None => {
                    out.push('<');
                    rest = &rest[1..];
                }
            }
        } else {
            let (text, len) = decode_entity(rest);
            out.push_str(text);
            rest = &rest[len..];
        }
    }
    out.push_str(rest);
}

/// The byte length of an HTML tag at the start of `text` (which begins with
/// `<`), and whether it is a line break — `None` when it does not look like
/// a tag (a bare `<`, a comparison, a generic type outside code).
fn tag_end(text: &str) -> Option<(usize, bool)> {
    let inner = text.strip_prefix('<')?;
    let inner = inner.strip_prefix('/').unwrap_or(inner);
    if !inner.starts_with(|c: char| c.is_ascii_alphabetic()) && !inner.starts_with('!') {
        return None;
    }
    let close = text.find('>')?;
    let name: String = text[1..close]
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    Some((close + 1, name.eq_ignore_ascii_case("br")))
}

/// Decode one entity at the start of `text` (which begins with `&`),
/// returning the replacement and the consumed byte length.
fn decode_entity(text: &str) -> (&'static str, usize) {
    const ENTITIES: [(&str, &str); 6] = [
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&nbsp;", " "),
    ];
    for (entity, replacement) in ENTITIES {
        if text.starts_with(entity) {
            return (replacement, entity.len());
        }
    }
    ("&", 1)
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn keeps_text_inside_tags() {
        assert_eq!(
            sanitize("<center><b>Sodium</b> is fast</center>"),
            "Sodium is fast"
        );
    }

    #[test]
    fn drops_void_tags_and_breaks_lines() {
        assert_eq!(sanitize("a<br>b"), "a\nb");
        assert_eq!(sanitize("badge <img src=\"x.png\"> here"), "badge  here");
    }

    #[test]
    fn leaves_code_alone() {
        assert_eq!(sanitize("use `Vec<String>` here"), "use `Vec<String>` here");
        assert_eq!(
            sanitize("```rust\nlet x: Vec<u8> = &v;\n```\n"),
            "```rust\nlet x: Vec<u8> = &v;\n```\n"
        );
    }

    #[test]
    fn keeps_bare_angle_brackets_and_ampersands() {
        assert_eq!(sanitize("1 < 2 && 3 > 2"), "1 < 2 && 3 > 2");
        assert_eq!(sanitize("a &amp; b &lt;ok&gt;"), "a & b <ok>");
    }
}

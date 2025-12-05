use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

pub fn parse_markdown(text: &str) -> Vec<Line<'_>> {
    let mut lines = Vec::new();

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();

        if trimmed.starts_with("# ") {
            // H1
            lines.push(Line::from(Span::styled(
                trimmed.trim_start_matches("# "),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
        } else if trimmed.starts_with("## ") {
            // H2
            lines.push(Line::from(Span::styled(
                trimmed.trim_start_matches("## "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("- ") {
            // List Item
            lines.push(Line::from(vec![
                Span::styled("• ", Style::default().fg(Color::Green)),
                parse_inline_styles(trimmed.trim_start_matches("- ")),
            ]));
        } else {
            // Normal Text
            lines.push(Line::from(parse_inline_styles(raw_line)));
        }
    }
    lines
}

fn parse_inline_styles(text: &str) -> Span<'_> {
    // A very simple parser for **bold**
    // For a full implementation, you'd return Vec<Span>
    if text.starts_with("**") && text.ends_with("**") && text.len() > 4 {
        Span::styled(
            &text[2..text.len() - 2],
            Style::default().add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(text)
    }
}

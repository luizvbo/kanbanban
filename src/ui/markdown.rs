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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::{Color, Modifier};

    #[test]
    fn test_parse_markdown_styles() {
        let input = "# Heading 1\n## Heading 2\n- List Item\nNormal text with **bold** content.";
        let lines = parse_markdown(input);

        assert_eq!(lines.len(), 4);

        // Test H1
        let h1 = &lines[0].spans[0];
        assert_eq!(h1.content, "Heading 1");
        assert_eq!(h1.style.fg, Some(Color::Yellow));
        assert!(h1.style.add_modifier.contains(Modifier::BOLD));

        // Test H2
        let h2 = &lines[1].spans[0];
        assert_eq!(h2.content, "Heading 2");
        assert_eq!(h2.style.fg, Some(Color::Cyan));

        // Test List
        let list_bullet = &lines[2].spans[0];
        let list_content = &lines[2].spans[1];
        assert_eq!(list_bullet.content, "• ");
        assert_eq!(list_content.content, "List Item");

        // Testing specific bold logic from code:
        let bold_input = "**Bold Text**";
        let bold_span = parse_inline_styles(bold_input);
        assert_eq!(bold_span.content, "Bold Text");
        assert!(bold_span.style.add_modifier.contains(Modifier::BOLD));

        let plain_input = "Plain Text";
        let plain_span = parse_inline_styles(plain_input);
        assert_eq!(plain_span.content, "Plain Text");
        assert!(plain_span.style.add_modifier.is_empty());
    }
}

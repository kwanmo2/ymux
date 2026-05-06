use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

const COLORS: [Color; 6] = [
    Color::Yellow,
    Color::Cyan,
    Color::Green,
    Color::Magenta,
    Color::Blue,
    Color::Red,
];

fn is_graph_char(c: char) -> bool {
    matches!(c, '|' | '*' | '/' | '\\' | '-' | '_' | '+')
}

/// Colorize one line of `git log --graph` output into a ratatui `Line`.
///
/// Graph-prefix characters are colored by lane position (every 2 cols cycles
/// through the palette). Everything after the graph prefix is rendered plain.
pub fn colorize(raw: &str) -> Line<'static> {
    let chars: Vec<char> = raw.chars().collect();
    let n = chars.len();

    // Graph prefix = leading graph chars + spaces
    let prefix_len = chars
        .iter()
        .position(|&c| !is_graph_char(c) && c != ' ')
        .unwrap_or(n);

    // Byte offset where the commit message starts
    let prefix_byte_end = raw
        .char_indices()
        .nth(prefix_len)
        .map(|(i, _)| i)
        .unwrap_or(raw.len());

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Group adjacent chars with the same (color, is_graph) into one span
    let mut run_text = String::new();
    let mut run_is_graph = false;
    let mut run_col = 0usize;

    for (i, &ch) in chars[..prefix_len].iter().enumerate() {
        let col = (i / 2) % COLORS.len();
        let is_g = is_graph_char(ch);

        if run_text.is_empty() {
            run_text.push(ch);
            run_is_graph = is_g;
            run_col = col;
        } else if col == run_col && is_g == run_is_graph {
            run_text.push(ch);
        } else {
            flush_span(&mut spans, &mut run_text, run_is_graph, run_col);
            run_text.push(ch);
            run_is_graph = is_g;
            run_col = col;
        }
    }
    flush_span(&mut spans, &mut run_text, run_is_graph, run_col);

    if prefix_byte_end < raw.len() {
        spans.push(Span::raw(raw[prefix_byte_end..].to_owned()));
    }

    Line::from(spans)
}

fn flush_span(spans: &mut Vec<Span<'static>>, text: &mut String, is_graph: bool, col: usize) {
    if text.is_empty() {
        return;
    }
    let s = text.clone();
    text.clear();
    if is_graph {
        spans.push(Span::styled(s, Style::default().fg(COLORS[col])));
    } else {
        spans.push(Span::raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_line_produces_no_spans() {
        let line = colorize("");
        assert!(line.spans.is_empty());
    }

    #[test]
    fn pure_message_is_single_span() {
        let line = colorize("abc1234 some commit");
        assert_eq!(line.spans.len(), 1);
        assert_eq!(line.spans[0].content, "abc1234 some commit");
    }

    #[test]
    fn star_prefix_splits_graph_and_message() {
        let line = colorize("* abc1234 some commit");
        // at minimum: the '*' span and the message span
        assert!(line.spans.len() >= 2);
    }

    #[test]
    fn complex_graph_prefix() {
        let line = colorize("| * def5678 branch commit");
        assert!(line.spans.len() >= 2);
        // All spans together reconstruct the original line
        let reconstructed: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(reconstructed, "| * def5678 branch commit");
    }

    #[test]
    fn graph_only_line_no_message_span() {
        // A line with only graph chars (e.g. "|/") has no trailing message
        let line = colorize("|/");
        let reconstructed: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(reconstructed, "|/");
    }

    #[test]
    fn roundtrip_preserves_text() {
        let cases = [
            "* abc1234 (HEAD -> main) initial commit",
            "| * def5678 feature work",
            "|/",
            "* | ghi9012 merge",
            "|\\",
        ];
        for raw in &cases {
            let line = colorize(raw);
            let out: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(&out, raw, "roundtrip failed for: {raw}");
        }
    }
}

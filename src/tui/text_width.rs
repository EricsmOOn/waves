use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

pub fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

pub fn truncate_to_width(text: &str, width: usize) -> String {
    if display_width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }

    let mut out = String::new();
    let budget = width - 1;
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let next = UnicodeWidthStr::width(grapheme);
        if used + next > budget {
            break;
        }
        out.push_str(grapheme);
        used += next;
    }
    out.push('…');
    out
}

pub fn pad_to_width(text: &str, width: usize, align: Align) -> String {
    let current = display_width(text);
    if current >= width {
        return truncate_to_width(text, width);
    }
    let pad = " ".repeat(width - current);
    match align {
        Align::Left => format!("{text}{pad}"),
        Align::Right => format!("{pad}{text}"),
    }
}

pub fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut line_width = 0;
    let mut pending_ascii_word = String::new();

    for grapheme in text.graphemes(true) {
        if grapheme == "\n" {
            flush_word(
                &mut pending_ascii_word,
                &mut line,
                &mut line_width,
                width,
                &mut lines,
            );
            lines.push(std::mem::take(&mut line));
            line_width = 0;
            continue;
        }

        if grapheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            pending_ascii_word.push_str(grapheme);
            continue;
        }

        flush_word(
            &mut pending_ascii_word,
            &mut line,
            &mut line_width,
            width,
            &mut lines,
        );
        push_grapheme(grapheme, &mut line, &mut line_width, width, &mut lines);
    }
    flush_word(
        &mut pending_ascii_word,
        &mut line,
        &mut line_width,
        width,
        &mut lines,
    );
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

fn flush_word(
    word: &mut String,
    line: &mut String,
    line_width: &mut usize,
    width: usize,
    lines: &mut Vec<String>,
) {
    if word.is_empty() {
        return;
    }
    let word_width = display_width(word);
    if *line_width > 0 && *line_width + word_width > width {
        lines.push(std::mem::take(line));
        *line_width = 0;
    }
    if word_width <= width {
        line.push_str(word);
        *line_width += word_width;
    } else {
        for grapheme in word.graphemes(true) {
            push_grapheme(grapheme, line, line_width, width, lines);
        }
    }
    word.clear();
}

fn push_grapheme(
    grapheme: &str,
    line: &mut String,
    line_width: &mut usize,
    width: usize,
    lines: &mut Vec<String>,
) {
    let grapheme_width = grapheme
        .chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum::<usize>();
    if *line_width > 0 && *line_width + grapheme_width > width {
        lines.push(std::mem::take(line));
        *line_width = 0;
    }
    line.push_str(grapheme);
    *line_width += grapheme_width;
}

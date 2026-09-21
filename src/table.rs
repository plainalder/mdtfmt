use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug)]
pub struct FormatError {
    message: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FormatError {}

struct Table {
    header: Vec<String>,
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
}

/// Rewrites every markdown table found in `input` with aligned columns and
/// consistent pipe placement. Lines outside of a table are passed through
/// unchanged, so this can safely run over a whole document.
pub fn format_document(input: &str, lenient: bool) -> Result<String, FormatError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < lines.len() {
        if is_table_row(lines[i]) && i + 1 < lines.len() && is_separator_row(lines[i + 1]) {
            let start = i;
            let mut end = i + 2;
            while end < lines.len() && is_table_row(lines[end]) {
                end += 1;
            }
            let table = parse_table(&lines[start..end], lenient)?;
            out.push_str(&render_table(&table));
            i = end;
        } else {
            out.push_str(lines[i]);
            out.push('\n');
            i += 1;
        }
    }
    Ok(out)
}

fn is_table_row(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.contains('|')
}

fn is_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('-') && split_row(trimmed).iter().all(|cell| is_separator_cell(cell))
}

fn is_separator_cell(cell: &str) -> bool {
    let bytes = cell.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut start = 0;
    let mut end = bytes.len();
    if bytes[start] == b':' {
        start += 1;
    }
    if end > start && bytes[end - 1] == b':' {
        end -= 1;
    }
    start < end && bytes[start..end].iter().all(|&b| b == b'-')
}

/// Splits a row on unescaped `|`, dropping the empty cell produced by a
/// leading or trailing pipe (`| a | b |` and `a | b` mean the same thing).
fn split_row(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.trim().chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            current.push('|');
            chars.next();
        } else if c == '|' {
            cells.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(c);
        }
    }
    cells.push(current.trim().to_string());

    if cells.first().is_some_and(|c| c.is_empty()) {
        cells.remove(0);
    }
    if cells.last().is_some_and(|c| c.is_empty()) {
        cells.pop();
    }
    cells
}

fn parse_alignment(cell: &str) -> Alignment {
    match (cell.starts_with(':'), cell.ends_with(':')) {
        (true, true) => Alignment::Center,
        (true, false) => Alignment::Left,
        (false, true) => Alignment::Right,
        (false, false) => Alignment::None,
    }
}

fn parse_table(block: &[&str], lenient: bool) -> Result<Table, FormatError> {
    let header = split_row(block[0]);
    let separator = split_row(block[1]);

    if !lenient && header.len() != separator.len() {
        return Err(FormatError {
            message: format!(
                "header has {} column(s) but the separator row has {} (pass --lenient to pad or truncate instead of failing)",
                header.len(),
                separator.len()
            ),
        });
    }

    let width = header.len();
    let alignments = resize(
        separator.iter().map(|c| parse_alignment(c)).collect(),
        width,
        Alignment::None,
    );

    let mut rows = Vec::with_capacity(block.len().saturating_sub(2));
    for (offset, line) in block[2..].iter().enumerate() {
        let cells = split_row(line);
        if !lenient && cells.len() != width {
            return Err(FormatError {
                message: format!(
                    "row {} has {} column(s), expected {} (pass --lenient to pad or truncate instead of failing)",
                    offset + 1,
                    cells.len(),
                    width
                ),
            });
        }
        rows.push(resize(cells, width, String::new()));
    }

    Ok(Table {
        header: resize(header, width, String::new()),
        alignments,
        rows,
    })
}

fn resize<T: Clone>(mut v: Vec<T>, len: usize, fill: T) -> Vec<T> {
    v.resize(len, fill);
    v
}

fn render_table(table: &Table) -> String {
    let mut widths: Vec<usize> = table.header.iter().map(|h| display_width(h)).collect();
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(display_width(cell));
        }
    }
    for w in widths.iter_mut() {
        *w = (*w).max(3);
    }

    let mut out = String::new();
    out.push_str(&render_row(&table.header, &widths, &table.alignments));
    out.push_str(&render_separator(&widths, &table.alignments));
    for row in &table.rows {
        out.push_str(&render_row(row, &widths, &table.alignments));
    }
    out
}

fn render_row(cells: &[String], widths: &[usize], alignments: &[Alignment]) -> String {
    let mut out = String::from("|");
    let empty = String::new();
    for (i, width) in widths.iter().enumerate() {
        let cell = cells.get(i).unwrap_or(&empty);
        let alignment = alignments.get(i).copied().unwrap_or(Alignment::None);
        out.push(' ');
        out.push_str(&pad(cell, *width, alignment));
        out.push(' ');
        out.push('|');
    }
    out.push('\n');
    out
}

fn render_separator(widths: &[usize], alignments: &[Alignment]) -> String {
    let mut out = String::from("|");
    for (width, alignment) in widths.iter().zip(alignments.iter()) {
        let dashes = "-".repeat(*width);
        let cell = match alignment {
            Alignment::Left => format!(":{}", &dashes[1..]),
            Alignment::Right => format!("{}:", &dashes[..dashes.len() - 1]),
            Alignment::Center => format!(":{}:", &dashes[1..dashes.len() - 1]),
            Alignment::None => dashes,
        };
        out.push(' ');
        out.push_str(&cell);
        out.push(' ');
        out.push('|');
    }
    out.push('\n');
    out
}

fn pad(cell: &str, width: usize, alignment: Alignment) -> String {
    let space = width.saturating_sub(display_width(cell));
    match alignment {
        Alignment::Right => format!("{}{}", " ".repeat(space), cell),
        Alignment::Center => {
            let left = space / 2;
            let right = space - left;
            format!("{}{}{}", " ".repeat(left), cell, " ".repeat(right))
        }
        Alignment::Left | Alignment::None => format!("{}{}", cell, " ".repeat(space)),
    }
}

/// Terminal column width of `s`, counting CJK and other East Asian Wide
/// characters as 2 columns and combining marks as 0, instead of the 1
/// column per `char` that `.chars().count()` assumes. Column widths need
/// this so a table with, say, Chinese headers still lines up.
fn display_width(s: &str) -> usize {
    s.chars().map(char_width).sum()
}

fn char_width(c: char) -> usize {
    if is_zero_width(c) {
        0
    } else if is_wide(c) {
        2
    } else {
        1
    }
}

fn is_zero_width(c: char) -> bool {
    let cp = c as u32;
    matches!(cp, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
        || matches!(c, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}')
}

/// Ranges taken from the "Wide" and "Fullwidth" categories of Unicode's
/// East Asian Width property (the same ones `wcwidth` treats as 2 columns
/// wide on a terminal).
fn is_wide(c: char) -> bool {
    let cp = c as u32;
    matches!(
        cp,
        0x1100..=0x115F
            | 0x2E80..=0x303E
            | 0x3041..=0x33FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA000..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
            | 0x20000..=0x3FFFD
    )
}

#[cfg(test)]
mod parsing_tests {
    use super::*;

    #[test]
    fn split_row_strips_leading_and_trailing_pipes() {
        assert_eq!(split_row("| a | b |"), vec!["a", "b"]);
        assert_eq!(split_row("a | b"), vec!["a", "b"]);
    }

    #[test]
    fn split_row_keeps_escaped_pipes_inside_a_cell() {
        assert_eq!(split_row(r"| a\|b | c |"), vec!["a|b", "c"]);
    }

    #[test]
    fn split_row_preserves_empty_cells_in_the_middle() {
        assert_eq!(split_row("| a || c |"), vec!["a", "", "c"]);
    }

    #[test]
    fn is_separator_row_accepts_alignment_markers() {
        assert!(is_separator_row("| --- | :--- | ---: | :---: |"));
    }

    #[test]
    fn is_separator_row_rejects_a_row_with_real_content() {
        assert!(!is_separator_row("| Name | --- |"));
    }

    #[test]
    fn parse_alignment_reads_colon_placement() {
        assert_eq!(parse_alignment("---"), Alignment::None);
        assert_eq!(parse_alignment(":---"), Alignment::Left);
        assert_eq!(parse_alignment("---:"), Alignment::Right);
        assert_eq!(parse_alignment(":---:"), Alignment::Center);
    }

    #[test]
    fn strict_mode_rejects_a_separator_with_the_wrong_column_count() {
        let block = ["| a | b |", "| --- |"];
        let err = parse_table(&block, false).unwrap_err();
        assert!(err.to_string().contains("separator row has 1"));
    }

    #[test]
    fn strict_mode_rejects_a_ragged_body_row() {
        let block = ["| a | b |", "| --- | --- |", "| 1 |"];
        let err = parse_table(&block, false).unwrap_err();
        assert!(err.to_string().contains("row 1 has 1 column"));
    }

    #[test]
    fn lenient_mode_pads_a_short_row_with_empty_cells() {
        let block = ["| a | b | c |", "| --- | --- | --- |", "| 1 | 2 |"];
        let table = parse_table(&block, true).unwrap();
        assert_eq!(table.rows[0], vec!["1", "2", ""]);
    }

    #[test]
    fn lenient_mode_truncates_a_long_row() {
        let block = ["| a | b |", "| --- | --- |", "| 1 | 2 | 3 |"];
        let table = parse_table(&block, true).unwrap();
        assert_eq!(table.rows[0], vec!["1", "2"]);
    }

    #[test]
    fn lenient_mode_tolerates_a_mismatched_separator() {
        let block = ["| a | b | c |", "| --- |"];
        let table = parse_table(&block, true).unwrap();
        assert_eq!(table.alignments.len(), 3);
        assert_eq!(table.alignments[1], Alignment::None);
    }

    #[test]
    fn format_document_passes_non_table_lines_through_unchanged() {
        let input = "# Title\n\nSome prose here.\n";
        assert_eq!(format_document(input, false).unwrap(), input);
    }

    #[test]
    fn format_document_formats_every_table_in_a_document() {
        let input = "\
Intro text.

| a | b |
|---|---|
| 1 | 2 |

Middle text.

| x |
|---|
| y |
";
        let output = format_document(input, false).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "Intro text.");
        assert_eq!(lines[2], "| a   | b   |");
        assert_eq!(lines[6], "Middle text.");
        assert_eq!(lines[8], "| x   |");
    }

    #[test]
    fn strict_mode_error_propagates_through_format_document() {
        let input = "| a | b |\n|---|---|\n| 1 |\n";
        assert!(format_document(input, false).is_err());
    }
}

#[cfg(test)]
mod width_tests {
    use super::*;

    #[test]
    fn ascii_is_one_column_per_char() {
        assert_eq!(display_width("abc"), 3);
    }

    #[test]
    fn cjk_is_two_columns_per_char() {
        assert_eq!(display_width("你好"), 4);
        assert_eq!(display_width("名前"), 4);
    }

    #[test]
    fn mixed_ascii_and_cjk() {
        assert_eq!(display_width("id: 你好"), 8);
    }

    #[test]
    fn combining_marks_are_zero_width() {
        // "e" + combining acute accent
        assert_eq!(display_width("e\u{0301}"), 1);
    }

    #[test]
    fn table_with_cjk_header_aligns_by_display_width() {
        let input = "| 名前 | Score |\n|---|---|\n| Ada | 98 |\n";
        let output = format_document(input, false).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        // "名前" is 4 columns wide, same as the widest cell below it ("Ada").
        assert_eq!(lines[0], "| 名前 | Score |");
        assert_eq!(lines[2], "| Ada  | 98    |");
    }
}

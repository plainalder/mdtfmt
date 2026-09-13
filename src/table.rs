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
    let mut widths: Vec<usize> = table.header.iter().map(|h| h.chars().count()).collect();
    for row in &table.rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
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
    let space = width.saturating_sub(cell.chars().count());
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

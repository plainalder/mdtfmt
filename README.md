# mdtfmt

A command-line tool that normalizes markdown tables: it reads a document,
finds every table in it, and rewrites each one with aligned columns and
consistent pipe placement. Everything that isn't a table passes through
untouched.

## Why

Markdown tables are edited by hand and it shows. Columns drift out of
alignment as content changes, some rows are missing a trailing pipe, some
have one column too many because of a stray `|` in a cell. Most tools that
"fix" this quietly pad or truncate whatever they find, which hides the fact
that the table was broken in the first place.

`mdtfmt` is strict by default: a row with the wrong number of columns is a
hard error, not something to paper over. When you do want the old
"whatever fits" behavior, pass `--lenient` and it will pad short rows with
empty cells and drop extra cells from long ones instead of failing.

## Example

Input (`notes.md`):

```
| Name | Score | Notes |
|---|---:|---|
| Ada | 98 | great |
| Grace | 87 |
```

Strict mode refuses to guess what the missing cell in the last row should
be:

```
$ mdtfmt notes.md
mdtfmt: row 2 has 2 column(s), expected 3 (pass --lenient to pad or truncate instead of failing)
```

With `--lenient`, the missing cell is padded with an empty string and the
table is reformatted:

```
$ mdtfmt --lenient notes.md
| Name  | Score | Notes |
| ----- | ----: | ----- |
| Ada   |    98 | great |
| Grace |    87 |       |
```

Column alignment markers in the separator row (`:---`, `---:`, `:---:`)
are read from the input and preserved.

## Usage

```
mdtfmt [--lenient] [--in-place] [FILE]
```

Reads `FILE`, or standard input if `FILE` is omitted or `-`. By default the
formatted document is written to standard output. Pass `--in-place` to
write the result back to `FILE` instead; this requires a real file
argument, since there's nowhere to write back to when reading from stdin.

## Status

Early skeleton. It handles single tables with a header, a separator row,
and body rows, including alignment markers and escaped pipes (`\|`) inside
cells. It does not yet handle tables with multi-line cells or wide (CJK)
character widths correctly.

## License

MIT, see [LICENSE](LICENSE).

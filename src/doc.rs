use crate::Language;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Document {
    path: PathBuf,
    content: String,
    tree: tree_sitter::Tree,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Match {
    id: u32,
    pattern: usize,
    captures: Vec<Capture>,
}

impl Match {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn pattner_index(&self) -> usize {
        self.pattern
    }
    pub fn captures(&self) -> impl Iterator<Item = Capture> + '_ {
        self.captures.iter().cloned()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Capture {
    index: u32,
    name: String,
    start: tree_sitter::Point,
    end: tree_sitter::Point,
    text: String,
}

impl Capture {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn start_position(&self) -> &tree_sitter::Point {
        &self.start
    }
    pub fn end_position(&self) -> &tree_sitter::Point {
        &self.end
    }
    pub fn text(&self) -> &str {
        &self.text.as_str()
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {}

impl Document {
    pub fn open<P: AsRef<Path>>(path: P, lang: &Language) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&lang.language())
            .map_err(std::io::Error::other)?;
        let tree = parser
            .parse(&content, None)
            .ok_or_else(|| std::io::Error::other("failed to parse"))?;
        Ok(Self {
            path: path.as_ref().to_owned(),
            content,
            tree,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.content.lines()
    }

    pub fn find(&self, query: &crate::Query) -> Result<impl Iterator<Item = Match>> {
        // TODO: return an iterator instead of making a copy of everything here.
        let mut qcursor = tree_sitter::QueryCursor::new();
        let matches = qcursor.matches(&query.query, self.tree.root_node(), self.content.as_bytes());
        let matches = matches
            .map(|m| Match {
                id: m.id(),
                pattern: m.pattern_index,
                captures: m
                    .captures
                    .into_iter()
                    .map(|c| Capture {
                        index: c.index,
                        name: query.capture_name(c.index).to_owned(),
                        start: c.node.start_position(),
                        end: c.node.end_position(),
                        text: c
                            .node
                            .utf8_text(self.content.as_bytes())
                            .unwrap_or_default()
                            .to_owned(),
                    })
                    .collect(),
            })
            .collect::<Vec<_>>()
            .into_iter();
        Ok(matches)
    }

    pub fn write_tree(&self, mut out: &mut impl std::io::Write) -> std::io::Result<()> {
        let mut cursor = self.tree.walk();
        let mut needs_newline = false;
        let mut indent_level = 0;
        let mut did_visit_children = false;
        loop {
            let node = cursor.node();
            let is_named = node.is_named();
            if did_visit_children {
                if is_named {
                    out.write_all(b")")?;
                    needs_newline = true;
                }
                if cursor.goto_next_sibling() {
                    did_visit_children = false;
                } else if cursor.goto_parent() {
                    did_visit_children = true;
                    indent_level -= 1;
                } else {
                    break;
                }
            } else {
                if is_named {
                    if needs_newline {
                        out.write_all(b"\n")?;
                    }
                    for _ in 0..indent_level {
                        out.write_all(b"  ")?;
                    }
                    let start = node.start_position();
                    let end = node.end_position();
                    if let Some(field_name) = cursor.field_name() {
                        write!(&mut out, "{field_name}: ")?;
                    }
                    write!(
                        &mut out,
                        "({} [{}, {}] - [{}, {}]",
                        node.kind(),
                        start.row,
                        start.column,
                        end.row,
                        end.column
                    )?;
                    needs_newline = true;
                }
                if cursor.goto_first_child() {
                    did_visit_children = false;
                    indent_level += 1;
                } else {
                    did_visit_children = true;
                }
            }
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("document error")
    }
}

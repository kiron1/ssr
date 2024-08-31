use crate::Language;
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub struct Document {
    path: PathBuf,
    lang: Language,
    content: String,
    // parser: tree_sitter::Parser,
    tree: tree_sitter::Tree,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Match {
    id: u32,
    pattern: usize,
    captures: Vec<Capture>,
}

impl Match {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn pattern_index(&self) -> usize {
        self.pattern
    }
    pub fn captures(&self) -> impl Iterator<Item = Capture> + '_ {
        self.captures.iter().cloned()
    }
}

impl rhai::CustomType for Match {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("Match")
            .with_get("id", |this: &mut Self| this.id())
            .with_get("pattern_index", |this: &mut Self| this.pattern_index())
            .with_get("captures", |this: &mut Self| -> rhai::Dynamic {
                this.captures.clone().into()
            });
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capture {
    index: u32,
    name: String,
    text: String,
    range: tree_sitter::Range,
}

impl Capture {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn range(&self) -> &tree_sitter::Range {
        &self.range
    }
    pub fn start_position(&self) -> &tree_sitter::Point {
        &self.range.start_point
    }
    pub fn end_position(&self) -> &tree_sitter::Point {
        &self.range.end_point
    }
    pub fn text(&self) -> &str {
        self.text.as_str()
    }
}

impl rhai::CustomType for Capture {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("Capture")
            .on_debug(|this: &mut Self| format!("{:?}", &this))
            .on_print(|this: &mut Self| {
                format!(
                    "@{} {} {}",
                    this.name(),
                    this.range.start_point,
                    this.range.end_point,
                )
            })
            .with_get("index", |this: &mut Self| this.index())
            .with_get("name", |this: &mut Self| this.name().to_owned())
            .with_get("range", |this: &mut Self| this.range().to_owned())
            .with_get("text", |this: &mut Self| this.text().to_owned());
    }
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Compiling error: {0}")]
    Compile(String),
    #[error("Script error in {0}: {1}")]
    Script(PathBuf, String),
    #[error("Language error: {0}")]
    Language(
        #[from]
        #[source]
        tree_sitter::LanguageError,
    ),
    #[error("Failed to parse document")]
    ParsingFailed,
    #[error("Query error: {0}")]
    Query(
        #[from]
        #[source]
        crate::query::Error,
    ),
    #[error("I/O error in {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
}

impl Document {
    pub fn open<P: AsRef<Path>>(path: P, lang: Language) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            let p = path.as_ref().to_owned();
            Error::Io(p, e)
        })?;

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang.language())?;
        let tree = parser.parse(&content, None).ok_or(Error::ParsingFailed)?;

        Ok(Self {
            path: path.as_ref().to_owned(),
            lang,
            content,
            // parser,
            tree,
        })
    }

    pub fn with_content(path: PathBuf, lang: Language, content: String) -> Result<Self> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang.language())?;
        let tree = parser.parse(&content, None).ok_or(Error::ParsingFailed)?;

        Ok(Self {
            path,
            lang,
            content,
            // parser,
            tree,
        })
    }

    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    pub fn content(&self) -> &str {
        self.content.as_str()
    }

    pub fn lines(&self) -> impl Iterator<Item = String> {
        let vec = self
            .content
            .lines()
            .map(|s| s.to_owned())
            .collect::<Vec<_>>();
        vec.into_iter()
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
                    .iter()
                    .map(|c| Capture {
                        index: c.index,
                        name: query.capture_name(c.index).to_owned(),
                        range: c.node.range(),
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

    pub fn edit(&self, query: &str, script: &str) -> Result<Self> {
        let engine = {
            let mut engine = rhai::Engine::new();
            engine.build_type::<DocumentEdits>();
            engine.build_type::<crate::Match>();
            engine.build_type::<crate::Capture>();
            engine
        };
        let ast = engine
            .compile(script)
            .map_err(|e| Error::Compile(e.to_string()))?;
        let found = self
            .find(&crate::Query::new(&self.lang, query)?)?
            .collect::<Vec<_>>();

        let edits = DocumentEdits::default();
        let mut scope = rhai::Scope::new();
        scope.push("document", edits.clone());

        for m in found {
            scope.set_value("found", m);

            let _result = engine
                .eval_ast_with_scope::<rhai::Dynamic>(&mut scope, &ast)
                .map_err({
                    let p = self.path.to_owned();
                    |e| Error::Script(p, e.to_string())
                })?;
        }
        self.apply_edits(edits.changes())
    }

    fn apply_edits(&self, changes: impl Iterator<Item = Change>) -> Result<Self> {
        let changes = {
            let mut e = changes.collect::<Vec<_>>();
            // Sort edits in *reverse* by edit start position.
            e.sort_by(|b, a| a.range.start_byte.cmp(&b.range.start_byte));
            e
        };
        let mut content = self.content.clone();
        for edit in changes {
            // let new_lines = edit.replacement.bytes().filter(|c| *c == b'\n').count();

            // let new_end_row = if new_lines == 0 {
            //     edit.range.start_point.row + edit.replacement.len()
            // } else {
            //     edit.replacement
            //         .split('\n')
            //         .last()
            //         .unwrap_or_default()
            //         .len()
            // };

            // let new_end_position = tree_sitter::Point {
            //     row: new_end_row,
            //     column: edit.range.start_point.column + new_lines,
            // };
            // let input_edit = tree_sitter::InputEdit {
            //     start_byte: edit.range.start_byte,
            //     old_end_byte: edit.range.end_byte,
            //     new_end_byte: edit.replacement.len(),
            //     start_position: edit.range.start_point,
            //     old_end_position: edit.range.end_point,
            //     new_end_position,
            // };
            // self.tree.edit(&input_edit);
            content = {
                let mut t = content[0..edit.range.start_byte].to_owned();
                t.push_str(edit.replacement.as_str());
                t.push_str(&content[edit.range.end_byte..]);
                t
            };
        }
        // self.tree = self
        //     .parser
        //     .parse(&self.content, Some(&self.tree))
        //     .ok_or(Error::ParsingFailed)?;

        Self::with_content(self.path.to_owned(), self.lang, content)
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

    pub fn diff(&self, other: &Self) -> String {
        let a = format!("a/{}", self.path.display());
        let b = format!("b/{}", other.path.display());
        similar::TextDiff::from_lines(self.content.as_str(), other.content.as_str())
            .unified_diff()
            .context_radius(5)
            .header(a.as_str(), b.as_str())
            .to_string()
    }
}

#[derive(Debug, Clone)]
struct Change {
    range: tree_sitter::Range,
    replacement: String,
}

#[derive(Debug, Default, Clone)]
struct DocumentEdits {
    edits: Arc<Mutex<Vec<Change>>>,
}

impl DocumentEdits {
    fn changes(self) -> impl Iterator<Item = Change> {
        let mut e = self.edits.lock().unwrap();
        std::mem::take(&mut *e).into_iter()
    }
}

impl rhai::CustomType for DocumentEdits {
    fn build(mut builder: rhai::TypeBuilder<Self>) {
        builder
            .with_name("Document")
            .with_fn("edit", |this: &mut Self, range, replacement| {
                this.edits
                    .lock()
                    .unwrap()
                    .push(Change { range, replacement });
            });
    }
}

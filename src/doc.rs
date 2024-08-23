use crate::Language;
use std::path::{Path, PathBuf};

pub struct Document {
    path: PathBuf,
    content: String,
    parser: tree_sitter::Parser,
    tree: tree_sitter::Tree,
}

impl Document {
    pub fn open<P: AsRef<Path>>(path: P, lang: Language) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&lang.into())
            .map_err(std::io::Error::other)?;
        let tree = parser.parse(&content, None).expect("parse");
        Ok(Self {
            path: path.as_ref().to_owned(),
            content,
            parser,
            tree,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
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

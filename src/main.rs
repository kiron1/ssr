//
// Structured Search Replace (SSR)
//
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use ssr::{Document, Language, Query};

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T> = std::result::Result<T, Error>;

/// SSR - Strucuted Search and Replace.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Options {
    #[command(subcommand)]
    command: SsrCommand,
}

#[derive(Debug, Subcommand)]
enum SsrCommand {
    /// Show tree sitter CST of a file.
    Tree(Tree),

    /// Apply query against all files.
    Search(Search),

    /// Use query to search in all files and use replace command to replace matches.
    Replace(Replace),
}

#[derive(Debug, Args)]
struct Tree {
    /// Which language to use.
    #[arg(short, long)]
    language: Language,
    /// Files to apply the query to
    file: PathBuf,
}

#[derive(Debug, Clone, Args)]
struct QueryOptions {
    /// Which language to use.
    #[arg(short, long)]
    language: Language,
    /// Tree-Sitter query as s-expression:
    /// https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries
    #[arg(short = 'q', long = "query")]
    source: String,
}

impl QueryOptions {
    fn query(&self) -> std::result::Result<Query, ssr::QueryError> {
        Query::new(self.language, self.source.as_str())
    }
}

#[derive(Debug, Clone, Args)]
struct WalkOptions {
    /// File types to search for
    #[arg(short = 't', long = "type")]
    ftype: Option<String>,
    /// Add a new file type.
    #[arg(long = "type-add")]
    type_defs: Vec<String>,
    /// Paths to walk for files.
    paths: Vec<PathBuf>,
}

impl WalkOptions {
    fn walker(
        &self,
    ) -> std::result::Result<
        impl Iterator<Item = std::result::Result<ignore::DirEntry, ignore::Error>>,
        ignore::Error,
    > {
        let types = {
            let mut types = ignore::types::TypesBuilder::new();
            types.add_defaults();
            for tdef in self.type_defs.iter() {
                types.add_def(tdef.as_str())?;
            }
            if let Some(ftype) = &self.ftype {
                types.select(ftype.as_str());
            }
            types.build()?
        };

        let cwd = PathBuf::from(".");
        let mut paths = self.paths.iter().fuse();
        let mut w = ignore::WalkBuilder::new(paths.next().unwrap_or(&cwd));
        for p in paths {
            w.add(p);
        }
        w.types(types);
        let iter = w.build().filter(|p| {
            p.as_ref()
                .map(|p| {
                    if let Ok(m) = p.metadata() {
                        m.is_file()
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        });
        Ok(iter)
    }
}

#[derive(Debug, Args)]
struct Search {
    #[command(flatten)]
    query: QueryOptions,
    #[command(flatten)]
    walk: WalkOptions,
}

#[derive(Debug, Args)]
struct Replace {
    #[command(flatten)]
    query: QueryOptions,
    /// Replacement script.
    #[arg(short, long)]
    replacement: String,
    #[command(flatten)]
    walk: WalkOptions,
}

impl SsrCommand {
    fn run(&self) -> Result<std::process::ExitCode> {
        match self {
            Self::Tree(cmd) => cmd.run(),
            Self::Search(cmd) => cmd.run(),
            Self::Replace(cmd) => cmd.run(),
        }
    }
}

impl Tree {
    fn run(&self) -> Result<std::process::ExitCode> {
        let doc = Document::open(&self.file, self.language)?;
        let mut out = std::io::stdout().lock();
        doc.write_tree(&mut out)?;

        Ok(std::process::ExitCode::SUCCESS)
    }
}

impl Search {
    fn run(&self) -> Result<std::process::ExitCode> {
        let mut found = false;
        for p in self.walk.walker()? {
            let p = p?;
            let p = p.path();
            let doc = Document::open(p, self.query.language)?;

            let lw = (doc.lines().count() as f32).log10().floor() as usize;

            for m in doc.find(&self.query.query()?)? {
                found = true;
                for c in m.captures() {
                    println!(
                        "{}  capture: {} [{}]",
                        (0..lw).map(|_| ' ').collect::<String>(),
                        c.name(),
                        m.pattern_index()
                    );
                    for (k, line) in doc
                        .lines()
                        .skip(c.start_position().row)
                        .take(c.end_position().row - c.start_position().row + 1)
                        .enumerate()
                    {
                        println!("{:lw$}: {line}", k + c.start_position().row + 1)
                    }
                }
                println!();
            }
        }
        Ok(if found {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        })
    }
}

impl Replace {
    fn run(&self) -> Result<std::process::ExitCode> {
        let mut changed = false;
        for p in self.walk.walker()? {
            let p = p?;
            let p = p.path();
            let doc = Document::open(p, self.query.language)?;
            let new = doc.edit(&self.query.source, &self.replacement)?;
            let patch = doc.diff(&new);
            if patch.is_changed() {
                changed = true;
                println!("{}", &patch);
            }
        }
        Ok(if changed {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        })
    }
}

fn main() -> Result<std::process::ExitCode> {
    let options = Options::parse();

    options.command.run()
}

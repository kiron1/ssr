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

#[derive(Debug, Args)]
struct Search {
    #[command(flatten)]
    query: QueryOptions,
    /// List of files to apply the query to
    files: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct Replace {
    #[command(flatten)]
    query: QueryOptions,
    /// Replacement script.
    #[arg(short, long)]
    replacement: String,
    /// List of files to apply the query to
    files: Vec<PathBuf>,
}

impl SsrCommand {
    fn run(&self) -> Result<()> {
        match self {
            Self::Tree(cmd) => cmd.run(),
            Self::Search(cmd) => cmd.run(),
            Self::Replace(cmd) => cmd.run(),
        }
    }
}

impl Tree {
    fn run(&self) -> Result<()> {
        let doc = Document::open(&self.file, self.language)?;
        let mut out = std::io::stdout().lock();
        doc.write_tree(&mut out)?;

        Ok(())
    }
}

impl Search {
    fn run(&self) -> Result<()> {
        let doc = Document::open(&self.files[0], self.query.language)?;

        let lw = (doc.lines().count() as f32).log10().floor() as usize;

        for m in doc.find(&self.query.query()?)? {
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

        Ok(())
    }
}

impl Replace {
    fn run(&self) -> Result<()> {
        for p in &self.files {
            let doc = Document::open(p, self.query.language)?;
            let new = doc.edit(&self.query.source, &self.replacement)?;
            println!("{}", doc.diff(&new));
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let options = Options::parse();

    options.command.run()
}

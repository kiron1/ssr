use std::{fmt::Display, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Language {
    Bazel,
    Python,
    Rust,
}

#[derive(Debug)]
pub struct Error;

impl Language {
    pub(crate) fn language(&self) -> tree_sitter::Language {
        match self {
            Self::Bazel | Self::Python => tree_sitter_python::language(),
            Self::Rust => tree_sitter_rust::language(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        // https://github.com/BurntSushi/ripgrep/blob/master/crates/ignore/src/default_types.rs#L12
        match self {
            Self::Bazel => "bazel",
            Self::Python => "python",
            Self::Rust => "rust",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid language")
    }
}

impl std::error::Error for Error {}

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Language {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_ascii_lowercase();
        let ret = match s.as_str() {
            "bazel" => Self::Bazel,
            "python" => Self::Python,
            "rust" => Self::Rust,
            _ => return Err(Error),
        };
        Ok(ret)
    }
}

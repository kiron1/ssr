pub struct Query {
    pub(crate) query: tree_sitter::Query,
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    inner: tree_sitter::QueryError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, mut f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("query error: ")?;
        self.inner.fmt(&mut f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl Query {
    pub fn new(language: &crate::Language, source: &str) -> Result<Self> {
        let query = tree_sitter::Query::new(&language.language(), source)
            .map_err(|inner| Error { inner })?;
        Ok(Self { query })
    }

    pub fn capture_name(&self, index: u32) -> &str {
        self.query.capture_names()[index as usize]
    }
}

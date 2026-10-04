#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(usize)]
pub enum SyntaxLanguage {
    Bash,
    C,
    Cpp,
    Css,
    Diff,
    Go,
    JavaScript,
    Json,
    Markdown,
    Nix,
    Python,
    Rust,
    TypeScript,
    Yaml,
}

impl SyntaxLanguage {
    pub(crate) const ALL: [Self; 14] = [
        Self::Bash,
        Self::C,
        Self::Cpp,
        Self::Css,
        Self::Diff,
        Self::Go,
        Self::JavaScript,
        Self::Json,
        Self::Markdown,
        Self::Nix,
        Self::Python,
        Self::Rust,
        Self::TypeScript,
        Self::Yaml,
    ];

    pub(crate) const fn index(self) -> usize {
        self as usize
    }

    pub(crate) const fn config_name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Css => "css",
            Self::Diff => "diff",
            Self::Go => "go",
            Self::JavaScript => "javascript",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Nix => "nix",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Yaml => "yaml",
        }
    }
}

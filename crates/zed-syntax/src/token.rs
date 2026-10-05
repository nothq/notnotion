use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxTokenKind {
    Comment,
    Function,
    Keyword,
    Literal,
    Operator,
    Property,
    Punctuation,
    String,
    Variable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyntaxHighlight {
    pub range: Range<usize>,
    pub kind: SyntaxTokenKind,
}

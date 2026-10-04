mod language;
mod mermaid;
mod syntax;

pub(in crate::ui::board_workspace::page::editor) use language::{
    PAGE_CODE_LANGUAGES, PAGE_CODE_LANGUAGE_OPTION_STRIDE,
};
pub(crate) use syntax::PageCodeSyntaxCache;

pub(crate) use mermaid::PageMermaidDiagrams;
pub(in crate::ui::board_workspace::page::editor) use syntax::PageCodeSyntaxRenderer;

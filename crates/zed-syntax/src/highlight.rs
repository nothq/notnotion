use std::{
    cell::RefCell,
    sync::{Arc, OnceLock},
};

use tree_sitter_highlight::{Highlight, HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{SyntaxHighlight, SyntaxLanguage, SyntaxTokenKind};

const CAPTURES: &[(&str, SyntaxTokenKind)] = &[
    ("attribute", SyntaxTokenKind::Property),
    ("boolean", SyntaxTokenKind::Literal),
    ("comment", SyntaxTokenKind::Comment),
    ("constant", SyntaxTokenKind::Literal),
    ("constructor", SyntaxTokenKind::Function),
    ("diff", SyntaxTokenKind::Property),
    ("escape", SyntaxTokenKind::String),
    ("function", SyntaxTokenKind::Function),
    ("import", SyntaxTokenKind::Keyword),
    ("keyword", SyntaxTokenKind::Keyword),
    ("label", SyntaxTokenKind::Property),
    ("number", SyntaxTokenKind::Literal),
    ("operator", SyntaxTokenKind::Operator),
    ("preproc", SyntaxTokenKind::Keyword),
    ("property", SyntaxTokenKind::Property),
    ("punctuation", SyntaxTokenKind::Punctuation),
    ("selector", SyntaxTokenKind::String),
    ("string", SyntaxTokenKind::String),
    ("tag", SyntaxTokenKind::Property),
    ("type", SyntaxTokenKind::Function),
    ("variable", SyntaxTokenKind::Variable),
];

static CONFIGURATIONS: [OnceLock<Arc<HighlightConfiguration>>; SyntaxLanguage::ALL.len()] =
    [const { OnceLock::new() }; SyntaxLanguage::ALL.len()];

thread_local! {
    static HIGHLIGHTER: RefCell<Highlighter> = RefCell::new(Highlighter::new());
}

pub fn highlight_syntax(text: &str, syntax: SyntaxLanguage) -> Arc<[SyntaxHighlight]> {
    if text.is_empty() {
        return Arc::from([]);
    }
    let configuration = CONFIGURATIONS[syntax.index()].get_or_init(|| build_configuration(syntax));
    HIGHLIGHTER.with_borrow_mut(|highlighter| collect_highlights(highlighter, configuration, text))
}

fn build_configuration(syntax: SyntaxLanguage) -> Arc<HighlightConfiguration> {
    let highlights = highlight_query(syntax);
    let mut configuration =
        HighlightConfiguration::new(grammar(syntax), syntax.config_name(), &highlights, "", "")
            .expect("pinned highlight queries must match their pinned grammars");
    let capture_names = CAPTURES.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    configuration.configure(&capture_names);
    Arc::new(configuration)
}

fn highlight_query(syntax: SyntaxLanguage) -> std::borrow::Cow<'static, str> {
    let query = match syntax {
        SyntaxLanguage::Nix => {
            return format!(
                "{}\n(binding \"=\" @operator)",
                tree_sitter_nix::HIGHLIGHTS_QUERY
            )
            .into();
        }
        SyntaxLanguage::Bash => include_str!("../queries/bash.scm"),
        SyntaxLanguage::C => include_str!("../queries/c.scm"),
        SyntaxLanguage::Cpp => include_str!("../queries/cpp.scm"),
        SyntaxLanguage::Css => include_str!("../queries/css.scm"),
        SyntaxLanguage::Diff => include_str!("../queries/diff.scm"),
        SyntaxLanguage::Go => include_str!("../queries/go.scm"),
        SyntaxLanguage::JavaScript => include_str!("../queries/javascript.scm"),
        SyntaxLanguage::Json => include_str!("../queries/json.scm"),
        SyntaxLanguage::Markdown => include_str!("../queries/markdown.scm"),
        SyntaxLanguage::Python => include_str!("../queries/python.scm"),
        SyntaxLanguage::Rust => include_str!("../queries/rust.scm"),
        SyntaxLanguage::TypeScript => include_str!("../queries/typescript.scm"),
        SyntaxLanguage::Yaml => include_str!("../queries/yaml.scm"),
    };
    query.into()
}

fn grammar(syntax: SyntaxLanguage) -> tree_sitter::Language {
    match syntax {
        SyntaxLanguage::Bash => tree_sitter_bash::LANGUAGE.into(),
        SyntaxLanguage::C => tree_sitter_c::LANGUAGE.into(),
        SyntaxLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        SyntaxLanguage::Css => tree_sitter_css::LANGUAGE.into(),
        SyntaxLanguage::Diff => tree_sitter_diff::LANGUAGE.into(),
        SyntaxLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        SyntaxLanguage::JavaScript => tree_sitter_typescript::LANGUAGE_TSX.into(),
        SyntaxLanguage::Json => tree_sitter_json::LANGUAGE.into(),
        SyntaxLanguage::Markdown => tree_sitter_md::LANGUAGE.into(),
        SyntaxLanguage::Nix => tree_sitter_nix::LANGUAGE.into(),
        SyntaxLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SyntaxLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SyntaxLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SyntaxLanguage::Yaml => tree_sitter_yaml::LANGUAGE.into(),
    }
}

fn collect_highlights(
    highlighter: &mut Highlighter,
    configuration: &HighlightConfiguration,
    text: &str,
) -> Arc<[SyntaxHighlight]> {
    let events = highlighter
        .highlight(configuration, text.as_bytes(), None, |_| None)
        .expect("a configured native grammar must parse its source");
    let mut active = Vec::new();
    let mut highlights = Vec::new();
    for event in events {
        match event.expect("native syntax highlighting must not be cancelled") {
            HighlightEvent::Source { start, end } => {
                if let Some(kind) = active.last().copied() {
                    push_highlight(&mut highlights, start, end, kind);
                }
            }
            HighlightEvent::HighlightStart(Highlight(index)) => active.push(CAPTURES[index].1),
            HighlightEvent::HighlightEnd => {
                active.pop().expect("highlight events must be balanced");
            }
        }
    }
    highlights.into()
}

fn push_highlight(
    highlights: &mut Vec<SyntaxHighlight>,
    start: usize,
    end: usize,
    kind: SyntaxTokenKind,
) {
    if start == end {
        return;
    }
    if let Some(previous) = highlights.last_mut() {
        if previous.kind == kind && previous.range.end == start {
            previous.range.end = end;
            return;
        }
    }
    highlights.push(SyntaxHighlight {
        range: start..end,
        kind,
    });
}

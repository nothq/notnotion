mod cache;
mod palette;

use std::sync::Arc;

use gpui::App;
use gpui_components::text_input::TextInputHighlight;
use zed_syntax::{highlight_syntax, SyntaxHighlight, SyntaxLanguage};

use super::super::{AppearanceMode, CardPageEditableBlock};
use crate::ui::{surface::PageInputResources, view_actions::ViewNotifier};

pub(crate) use cache::PageCodeSyntaxCache;
use cache::{PageCodeSyntaxKey, PageCodeSyntaxLookup};
use palette::page_code_token_highlights;

#[derive(Clone)]
pub(in crate::ui::board_workspace::page::editor) struct PageCodeSyntaxRenderer {
    resources: PageInputResources,
    appearance_mode: AppearanceMode,
    notifier: ViewNotifier,
}

impl PageCodeSyntaxRenderer {
    pub(in crate::ui::board_workspace::page::editor) fn new(
        resources: PageInputResources,
        appearance_mode: AppearanceMode,
        notifier: ViewNotifier,
    ) -> Self {
        Self {
            resources,
            appearance_mode,
            notifier,
        }
    }

    pub(in crate::ui::board_workspace::page::editor) fn highlights(
        &self,
        block_id: &str,
        editable: &CardPageEditableBlock,
        cx: &mut App,
    ) -> Vec<TextInputHighlight> {
        let Some(key) = PageCodeSyntaxKey::from_editable(editable) else {
            self.resources
                .state()
                .code_syntax
                .borrow_mut()
                .remove(block_id);
            return Vec::new();
        };
        let lookup = self
            .resources
            .state()
            .code_syntax
            .borrow_mut()
            .lookup(block_id, &key);
        match lookup {
            PageCodeSyntaxLookup::Ready(highlights) => {
                page_code_token_highlights(&highlights, key.language, self.appearance_mode)
            }
            PageCodeSyntaxLookup::Pending => Vec::new(),
            PageCodeSyntaxLookup::Missing => {
                self.spawn_page_code_syntax(block_id.to_string(), key, cx);
                Vec::new()
            }
        }
    }
}

impl PageCodeSyntaxRenderer {
    fn spawn_page_code_syntax(&self, block_id: String, key: PageCodeSyntaxKey, cx: &mut App) {
        let request = key.clone();
        let work = cx
            .background_executor()
            .spawn(async move { highlight_syntax(request.text.as_ref(), request.language) });
        let completion_id = block_id.clone();
        let completion_key = key.clone();
        let resources = self.resources.downgrade();
        let appearance_mode = self.appearance_mode;
        let notifier = self.notifier.clone();
        let task = cx.spawn(async move |cx| {
            let highlights = work.await;
            cx.update(|cx| {
                if !notifier.is_alive() {
                    return;
                }
                let Some(resources) = resources.upgrade() else {
                    return;
                };
                Self::new(resources, appearance_mode, notifier).finish_page_code_syntax(
                    completion_id,
                    completion_key,
                    highlights,
                    cx,
                );
            });
        });
        self.resources
            .state()
            .code_syntax
            .borrow_mut()
            .begin(block_id, key, task);
    }

    fn finish_page_code_syntax(
        &self,
        block_id: String,
        key: PageCodeSyntaxKey,
        highlights: Arc<[SyntaxHighlight]>,
        cx: &mut App,
    ) {
        if !self
            .resources
            .state()
            .code_syntax
            .borrow_mut()
            .finish(&block_id, key, highlights)
        {
            return;
        }
        self.resources
            .state()
            .block_input_props
            .borrow_mut()
            .remove(&block_id);
        self.notifier.notify(cx);
    }
}

fn syntax_language(editable: &CardPageEditableBlock) -> Option<SyntaxLanguage> {
    match editable.code_language()?.as_str() {
        "Bash" | "Shell" => Some(SyntaxLanguage::Bash),
        "C" => Some(SyntaxLanguage::C),
        "C++" => Some(SyntaxLanguage::Cpp),
        "CSS" => Some(SyntaxLanguage::Css),
        "Diff" => Some(SyntaxLanguage::Diff),
        "Go" => Some(SyntaxLanguage::Go),
        "JavaScript" => Some(SyntaxLanguage::JavaScript),
        "JSON" => Some(SyntaxLanguage::Json),
        "Markdown" => Some(SyntaxLanguage::Markdown),
        "Nix" => Some(SyntaxLanguage::Nix),
        "Python" => Some(SyntaxLanguage::Python),
        "Rust" => Some(SyntaxLanguage::Rust),
        "TypeScript" => Some(SyntaxLanguage::TypeScript),
        "YAML" => Some(SyntaxLanguage::Yaml),
        _ => None,
    }
}

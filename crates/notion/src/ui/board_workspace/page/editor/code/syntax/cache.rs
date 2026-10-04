use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use gpui::Task;
use zed_syntax::{SyntaxHighlight, SyntaxLanguage};

use super::{syntax_language, CardPageEditableBlock};

const MAX_PAGE_CODE_SYNTAX_ENTRIES: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PageCodeSyntaxKey {
    pub(super) text: Arc<str>,
    pub(super) language: SyntaxLanguage,
}

impl PageCodeSyntaxKey {
    pub(super) fn from_editable(editable: &CardPageEditableBlock) -> Option<Self> {
        let language = syntax_language(editable)?;
        (!editable.text.is_empty()).then(|| Self {
            text: Arc::from(editable.text.as_str()),
            language,
        })
    }
}

pub(super) enum PageCodeSyntaxLookup {
    Ready(Arc<[SyntaxHighlight]>),
    Pending,
    Missing,
}

enum PageCodeSyntaxEntry {
    Pending {
        key: PageCodeSyntaxKey,
        _task: Task<()>,
    },
    Ready {
        key: PageCodeSyntaxKey,
        highlights: Arc<[SyntaxHighlight]>,
    },
}

impl PageCodeSyntaxEntry {
    fn key(&self) -> &PageCodeSyntaxKey {
        match self {
            Self::Pending { key, .. } | Self::Ready { key, .. } => key,
        }
    }
}

#[derive(Default)]
pub(crate) struct PageCodeSyntaxCache {
    entries: HashMap<String, PageCodeSyntaxEntry>,
    recency: VecDeque<String>,
}

impl PageCodeSyntaxCache {
    pub(super) fn lookup(
        &mut self,
        block_id: &str,
        key: &PageCodeSyntaxKey,
    ) -> PageCodeSyntaxLookup {
        let lookup = match self.entries.get(block_id) {
            Some(PageCodeSyntaxEntry::Ready {
                key: cached,
                highlights,
            }) if cached == key => PageCodeSyntaxLookup::Ready(highlights.clone()),
            Some(PageCodeSyntaxEntry::Pending { key: cached, .. }) if cached == key => {
                PageCodeSyntaxLookup::Pending
            }
            _ => PageCodeSyntaxLookup::Missing,
        };
        if !matches!(lookup, PageCodeSyntaxLookup::Missing) {
            self.touch(block_id);
        }
        lookup
    }

    pub(super) fn begin(&mut self, block_id: String, key: PageCodeSyntaxKey, task: Task<()>) {
        self.entries.insert(
            block_id.clone(),
            PageCodeSyntaxEntry::Pending { key, _task: task },
        );
        self.touch(&block_id);
        self.trim();
    }

    pub(super) fn finish(
        &mut self,
        block_id: &str,
        key: PageCodeSyntaxKey,
        highlights: Arc<[SyntaxHighlight]>,
    ) -> bool {
        let accepts = self.entries.get(block_id).is_some_and(|entry| {
            matches!(entry, PageCodeSyntaxEntry::Pending { .. }) && entry.key() == &key
        });
        if !accepts {
            return false;
        }
        self.entries.insert(
            block_id.to_string(),
            PageCodeSyntaxEntry::Ready { key, highlights },
        );
        self.touch(block_id);
        true
    }

    pub(crate) fn remove(&mut self, block_id: &str) {
        self.entries.remove(block_id);
        self.recency.retain(|candidate| candidate != block_id);
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.recency.clear();
    }

    pub(crate) fn cancel_pending(&mut self) {
        self.entries
            .retain(|_, entry| matches!(entry, PageCodeSyntaxEntry::Ready { .. }));
        self.recency
            .retain(|block_id| self.entries.contains_key(block_id));
    }

    pub(crate) fn retain(&mut self, mut retain: impl FnMut(&str) -> bool) {
        self.entries.retain(|block_id, _| retain(block_id));
        self.recency
            .retain(|block_id| self.entries.contains_key(block_id));
    }

    fn touch(&mut self, block_id: &str) {
        self.recency.retain(|candidate| candidate != block_id);
        self.recency.push_back(block_id.to_string());
    }

    fn trim(&mut self) {
        while self.entries.len() > MAX_PAGE_CODE_SYNTAX_ENTRIES {
            let Some(block_id) = self.recency.pop_front() else {
                break;
            };
            self.entries.remove(&block_id);
        }
    }
}

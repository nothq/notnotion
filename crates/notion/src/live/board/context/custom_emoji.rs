use std::{sync::Arc, time::Duration};

use crate::{
    live::board::{
        LiveCustomEmojiLibraryCache, LiveCustomEmojiLibraryFailure, LiveWorkspaceCache,
        LiveWorkspaceContext,
    },
    live::{credentials::NotionDesktopSession, NotionLiveError},
    model::NotionCustomEmojiLibrary,
};

const CUSTOM_EMOJI_LIBRARY_CACHE_TTL: Duration = Duration::from_secs(600);
const CUSTOM_EMOJI_LIBRARY_FAILURE_TTL: Duration = Duration::from_secs(1);

struct PendingCustomEmojiLibraryLoad {
    space_id: String,
    creation_allowed: bool,
    revision: u64,
}

enum CustomEmojiLibraryLookup {
    Cached(NotionCustomEmojiLibrary),
    Load(PendingCustomEmojiLibraryLoad),
}

impl LiveWorkspaceContext {
    pub(crate) fn load_custom_emoji_library(
        &self,
        session: &NotionDesktopSession,
    ) -> Result<NotionCustomEmojiLibrary, NotionLiveError> {
        loop {
            let pending = match self.custom_emoji_library_lookup()? {
                CustomEmojiLibraryLookup::Cached(library) => return Ok(library),
                CustomEmojiLibraryLookup::Load(pending) => pending,
            };
            let result = crate::live::load_notion_custom_emoji_library(
                session,
                &pending.space_id,
                pending.creation_allowed,
            );
            if let Some(library) = self.commit_custom_emoji_library_load(&pending, result)? {
                return Ok(library);
            }
        }
    }

    fn custom_emoji_library_lookup(&self) -> Result<CustomEmojiLibraryLookup, String> {
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        loop {
            if let Some(cached) = cache
                .custom_emoji_library
                .as_ref()
                .filter(|cached| cached.fetched_at.elapsed() < CUSTOM_EMOJI_LIBRARY_CACHE_TTL)
            {
                return Ok(CustomEmojiLibraryLookup::Cached(cached.library.clone()));
            }
            let revision = cache.custom_emoji_library_revision;
            if let Some(error) = recent_custom_emoji_library_failure(&cache, revision) {
                return Err(error);
            }
            cache.custom_emoji_library_failure = None;
            if cache.custom_emoji_library_loading_revision != Some(revision) {
                cache.custom_emoji_library_loading_revision = Some(revision);
                return Ok(CustomEmojiLibraryLookup::Load(
                    PendingCustomEmojiLibraryLoad {
                        space_id: cache.space_id.clone(),
                        creation_allowed: cache.user_context.can_create_custom_emoji,
                        revision,
                    },
                ));
            }
            let changed = Arc::clone(&cache.custom_emoji_library_changed);
            cache = changed.wait(cache).map_err(|_| {
                "Notion workspace cache lock is poisoned while waiting for custom emojis"
                    .to_string()
            })?;
        }
    }

    fn commit_custom_emoji_library_load(
        &self,
        pending: &PendingCustomEmojiLibraryLoad,
        result: Result<NotionCustomEmojiLibrary, NotionLiveError>,
    ) -> Result<Option<NotionCustomEmojiLibrary>, NotionLiveError> {
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        if cache.custom_emoji_library_loading_revision == Some(pending.revision) {
            cache.custom_emoji_library_loading_revision = None;
        }
        let changed = Arc::clone(&cache.custom_emoji_library_changed);
        if cache.space_id != pending.space_id {
            changed.notify_all();
            return Err(NotionLiveError::Fatal(
                "Notion workspace changed while loading custom emojis".to_string(),
            ));
        }
        if cache.custom_emoji_library_revision != pending.revision {
            changed.notify_all();
            return Ok(None);
        }
        let library = match result {
            Ok(library) => library,
            Err(NotionLiveError::Session(error)) => {
                changed.notify_all();
                return Err(NotionLiveError::Session(error));
            }
            Err(error) => {
                cache.custom_emoji_library_failure = Some(LiveCustomEmojiLibraryFailure {
                    failed_at: std::time::Instant::now(),
                    revision: pending.revision,
                    error: error.to_string(),
                });
                changed.notify_all();
                return Err(error);
            }
        };
        cache.custom_emoji_library_failure = None;
        cache.custom_emoji_library = Some(LiveCustomEmojiLibraryCache {
            fetched_at: std::time::Instant::now(),
            library: library.clone(),
        });
        changed.notify_all();
        Ok(Some(library))
    }

    pub(crate) fn ensure_custom_emoji_creation_allowed(&self) -> Result<(), String> {
        let cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        if !cache.user_context.can_create_custom_emoji {
            return Err(
                "the current Notion user cannot create workspace custom emojis".to_string(),
            );
        }
        Ok(())
    }

    pub(crate) fn invalidate_custom_emoji_library(&self) -> Result<(), String> {
        let mut cache = self
            .workspace_cache
            .lock()
            .map_err(|_| "Notion workspace cache lock is poisoned".to_string())?;
        cache.custom_emoji_library = None;
        cache.custom_emoji_library_revision = cache
            .custom_emoji_library_revision
            .checked_add(1)
            .expect("custom-emoji library revision overflowed");
        cache.custom_emoji_library_failure = None;
        let changed = Arc::clone(&cache.custom_emoji_library_changed);
        drop(cache);
        changed.notify_all();
        Ok(())
    }
}

fn recent_custom_emoji_library_failure(
    cache: &LiveWorkspaceCache,
    revision: u64,
) -> Option<String> {
    cache
        .custom_emoji_library_failure
        .as_ref()
        .filter(|failure| {
            failure.revision == revision
                && failure.failed_at.elapsed() < CUSTOM_EMOJI_LIBRARY_FAILURE_TTL
        })
        .map(|failure| failure.error.clone())
}

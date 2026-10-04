use std::sync::{Arc, Mutex};

use crate::model::{CardPageCodeLanguage, CardPageCodeSettings, CardPageCodeWrap};

pub(crate) trait CardPageCodeSettingsBackend: Send + Sync + 'static {
    fn current(&self) -> CardPageCodeSettings;
    fn stage_language(&self, language: CardPageCodeLanguage);
    fn stage_wrap(&self, wrap: CardPageCodeWrap);
}

#[derive(Clone)]
pub struct CardPageCodeSettingsCapability {
    backend: Arc<dyn CardPageCodeSettingsBackend>,
}

impl CardPageCodeSettingsCapability {
    pub(crate) fn new(backend: Arc<dyn CardPageCodeSettingsBackend>) -> Self {
        Self { backend }
    }

    pub(crate) fn memory() -> Self {
        Self::memory_with(CardPageCodeSettings::default())
    }

    pub(crate) fn memory_with(settings: CardPageCodeSettings) -> Self {
        Self::new(Arc::new(MemoryCodeSettingsBackend {
            settings: Mutex::new(settings),
        }))
    }

    pub fn current(&self) -> CardPageCodeSettings {
        self.backend.current()
    }

    pub fn stage_language(&self, language: CardPageCodeLanguage) {
        self.backend.stage_language(language);
    }

    pub fn stage_wrap(&self, wrap: CardPageCodeWrap) {
        self.backend.stage_wrap(wrap);
    }
}

struct MemoryCodeSettingsBackend {
    settings: Mutex<CardPageCodeSettings>,
}

impl CardPageCodeSettingsBackend for MemoryCodeSettingsBackend {
    fn current(&self) -> CardPageCodeSettings {
        self.settings
            .lock()
            .expect("in-memory Code settings lock must not be poisoned")
            .clone()
    }

    fn stage_language(&self, language: CardPageCodeLanguage) {
        let mut settings = self
            .settings
            .lock()
            .expect("in-memory Code settings lock must not be poisoned");
        *settings = settings.clone().with_language(language);
    }

    fn stage_wrap(&self, wrap: CardPageCodeWrap) {
        let mut settings = self
            .settings
            .lock()
            .expect("in-memory Code settings lock must not be poisoned");
        *settings = settings.clone().with_wrap(wrap);
    }
}

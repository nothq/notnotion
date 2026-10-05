use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::model::{
    CardPageCodeLanguage, CardPageCodeSettings, CardPageCodeSettingsBackend,
    CardPageCodeSettingsCapability, CardPageCodeWrap,
};

use super::credentials::NotionDesktopSession;

const NOTION_CACHE_DIR_ENV: &str = "NOTNOTION_CACHE_DIR";
const CODE_SETTINGS_KEY_NAMESPACE: &str = "notion|code-settings|v1";
const CODE_SETTINGS_SCHEMA_VERSION: u8 = 1;
const CODE_SETTINGS_MAX_BYTES: usize = 4_096;

#[derive(Default)]
pub(super) struct NotionCodePreferences {
    by_user: Mutex<HashMap<String, CardPageCodeSettingsCapability>>,
}

impl NotionCodePreferences {
    pub(super) fn for_session(
        &self,
        session: &NotionDesktopSession,
    ) -> CardPageCodeSettingsCapability {
        let user_id = session.user_id();
        let mut by_user = self
            .by_user
            .lock()
            .expect("Notion Code preference registry lock must not be poisoned");
        if let Some(settings) = by_user.get(user_id) {
            return settings.clone();
        }
        let settings = load_user_capability(user_id);
        by_user.insert(user_id.to_string(), settings.clone());
        settings
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCodeSettings {
    schema_version: u8,
    active_user_id: String,
    settings: CardPageCodeSettings,
}

#[derive(Clone)]
struct CodeSettingsStore {
    path: PathBuf,
    key: [u8; 32],
    active_user_id: String,
}

impl CodeSettingsStore {
    fn open(active_user_id: &str) -> Result<Self, String> {
        let cache_key = format!("{CODE_SETTINGS_KEY_NAMESPACE}|{active_user_id}");
        let key = local_cache::load_or_create_cache_key(&cache_key, "Notion Code settings")?;
        let root = local_cache::default_cache_root_dir(
            NOTION_CACHE_DIR_ENV,
            "notion",
            "Notion Code settings",
        )?;
        Ok(Self {
            path: local_cache::account_cache_dir(&root, &cache_key).join("code-settings.bin"),
            key,
            active_user_id: active_user_id.to_string(),
        })
    }

    fn load(&self) -> CardPageCodeSettings {
        let result = local_cache::read_encrypted_json_strict_bounded::<StoredCodeSettings>(
            &self.path,
            &self.key,
            CODE_SETTINGS_MAX_BYTES,
        );
        match result {
            Ok(None) => CardPageCodeSettings::default(),
            Ok(Some(stored)) if self.valid(&stored) => stored.settings,
            Ok(Some(_)) => {
                println!("notnotion Code preferences have an invalid identity or schema");
                CardPageCodeSettings::default()
            }
            Err(error) => {
                println!("notnotion Code preferences are unreadable: {error}");
                CardPageCodeSettings::default()
            }
        }
    }

    fn persist(&self, settings: &CardPageCodeSettings) {
        let stored = StoredCodeSettings {
            schema_version: CODE_SETTINGS_SCHEMA_VERSION,
            active_user_id: self.active_user_id.clone(),
            settings: settings.clone(),
        };
        if let Err(error) = local_cache::write_encrypted_json_durable_bounded(
            &self.path,
            &self.key,
            &stored,
            CODE_SETTINGS_MAX_BYTES,
        ) {
            println!("notnotion Code preference write failed: {error}");
        }
    }

    fn valid(&self, stored: &StoredCodeSettings) -> bool {
        stored.schema_version == CODE_SETTINGS_SCHEMA_VERSION
            && stored.active_user_id == self.active_user_id
    }
}

struct PersistentCodeSettingsBackend {
    current: Arc<Mutex<CardPageCodeSettings>>,
    sender: mpsc::SyncSender<()>,
}

impl PersistentCodeSettingsBackend {
    fn start(initial: CardPageCodeSettings, store: CodeSettingsStore) -> Result<Arc<Self>, String> {
        let current = Arc::new(Mutex::new(initial));
        let writer_current = current.clone();
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("notion-code-preferences".to_string())
            .spawn(move || persist_latest_settings(receiver, writer_current, store))
            .map_err(|error| format!("failed to start Notion Code preference writer: {error}"))?;
        Ok(Arc::new(Self { current, sender }))
    }

    fn stage(&self, update: impl FnOnce(CardPageCodeSettings) -> CardPageCodeSettings) {
        let mut current = self
            .current
            .lock()
            .expect("Notion Code preference state lock must not be poisoned");
        *current = update(current.clone());
        drop(current);
        match self.sender.try_send(()) {
            Ok(()) | Err(mpsc::TrySendError::Full(())) => {}
            Err(mpsc::TrySendError::Disconnected(())) => {
                println!("notnotion Code preference writer stopped unexpectedly");
            }
        }
    }
}

impl CardPageCodeSettingsBackend for PersistentCodeSettingsBackend {
    fn current(&self) -> CardPageCodeSettings {
        self.current
            .lock()
            .expect("Notion Code preference state lock must not be poisoned")
            .clone()
    }

    fn stage_language(&self, language: CardPageCodeLanguage) {
        self.stage(|settings| settings.with_language(language));
    }

    fn stage_wrap(&self, wrap: CardPageCodeWrap) {
        self.stage(|settings| settings.with_wrap(wrap));
    }
}

fn load_user_capability(active_user_id: &str) -> CardPageCodeSettingsCapability {
    let store = match CodeSettingsStore::open(active_user_id) {
        Ok(store) => store,
        Err(error) => {
            println!("notnotion Code preferences are unavailable: {error}");
            return CardPageCodeSettingsCapability::memory();
        }
    };
    let initial = store.load();
    match PersistentCodeSettingsBackend::start(initial.clone(), store) {
        Ok(backend) => CardPageCodeSettingsCapability::new(backend),
        Err(error) => {
            println!("notnotion Code preferences are memory-only: {error}");
            CardPageCodeSettingsCapability::memory_with(initial)
        }
    }
}

fn persist_latest_settings(
    receiver: mpsc::Receiver<()>,
    current: Arc<Mutex<CardPageCodeSettings>>,
    store: CodeSettingsStore,
) {
    while receiver.recv().is_ok() {
        while receiver.try_recv().is_ok() {}
        let settings = current
            .lock()
            .expect("Notion Code preference writer state lock must not be poisoned")
            .clone();
        store.persist(&settings);
    }
}

use anyhow::Result;
use midlang_core::store::{KVCursor, KVStore, MemStore, QueryCursor, RedbStore, SortOrder};
use std::path::Path;
pub struct PresetBuilder {
    inner: Box<MemStore>,
}

impl PresetBuilder {
    pub fn new() -> Self {
        Self {
            inner: Box::new(MemStore::new()),
        }
    }

    pub fn kv(&self, locale: &str, key: &str, value: &str) -> Result<&Self> {
        (self.inner.clone() as Box<dyn KVStore>).set(&locale, &key, &value)?;
        Ok(self)
    }

    pub fn export<S: KVStore>(&self, store: &S) -> Result<&Self> {
        let inner = self.inner.clone() as Box<dyn KVStore>;
        let locales = inner
            .statistics()?
            .per_locale
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        println!("Exporting locales: {:?}", locales);
        for locale in locales.iter() {
            let mut cursor: KVCursor = QueryCursor::new(SortOrder::Asc, None);
            loop {
                let page = inner.list(locale, &cursor, 1000)?;
                for entry in &page.items {
                    store.set(locale, &entry.key, &entry.value)?;
                }
                match page.next {
                    Some(next_cursor) => cursor = next_cursor,
                    None => break,
                }
            }
        }

        Ok(self)
    }
}

pub fn redb_store(path: impl AsRef<Path>) -> Result<RedbStore> {
    let store = RedbStore::new(path)?;
    Ok(store)
}

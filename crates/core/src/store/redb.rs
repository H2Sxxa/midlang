use super::KVStore;
pub struct RedbStore {}

impl KVStore for RedbStore {
    fn init(&mut self) {
        todo!()
    }
    fn set(&mut self, locale: &str, key: &str, value: &str) {
        todo!()
    }
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        todo!()
    }
}

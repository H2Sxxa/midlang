pub mod redb;
// "locale:namespace.key":"Value"
pub trait KVStore {
    fn init(&mut self);
    fn set(&mut self, locale: &str, key: &str, value: &str);
    fn get(&self, locale: &str, key: &str) -> Option<String>;
}

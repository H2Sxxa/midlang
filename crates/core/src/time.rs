/// Seconds since the Unix epoch.
///
/// Every instant column in the SQLite schema stores this value, so it is the
/// single source of "now" shared by the auth and internals services.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the Unix epoch")
        .as_secs() as i64
}

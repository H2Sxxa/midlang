use midlang_core::store::{KVCursor, KVRead, KVStore, RedbStore, SortOrder};
use redb::{Database, TableDefinition};
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const N: usize = 100_000;

fn temp_path(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "midlang-perf-{tag}-{}-{nanos}.rdb",
        std::process::id()
    ));
    path
}

fn seed(path: &PathBuf) {
    let db = Database::create(path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let tb: TableDefinition<'_, String, String> = TableDefinition::new("en");
        let mut table = tx.open_table(tb).unwrap();
        for i in 0..N {
            let key = format!("app.namespace.key{i:06}");
            let value = format!("The value number {i} of the translation entry");
            table.insert(key, value).unwrap();
        }
    }
    tx.commit().unwrap();
    drop(db);
}

fn cursor_of(keyword: Option<&str>) -> KVCursor {
    KVCursor::new(SortOrder::Asc, keyword.map(str::to_string))
}

fn page(store: &RedbStore, cursor: &KVCursor, limit: usize) -> (usize, Option<KVCursor>) {
    let start = Instant::now();
    let page = store.list("en", cursor, limit).unwrap();
    println!(
        "  filter={:?} limit={limit:<6} -> {:?} ({} rows, next={})",
        cursor.filter,
        start.elapsed(),
        page.items.len(),
        page.next.is_some()
    );
    (page.items.len(), page.next)
}

#[test]
fn probe() {
    let path = temp_path("query");
    let seed_start = Instant::now();
    seed(&path);
    println!(
        "seed {N} rows in one transaction: {:?}",
        seed_start.elapsed()
    );

    let store = RedbStore::new(&path).unwrap();

    println!("query:");
    page(&store, &cursor_of(None), 50);
    page(&store, &cursor_of(Some("key000001")), 50);
    page(&store, &cursor_of(Some("app.namespace.key099999")), 50);
    page(&store, &cursor_of(Some("nothing-matches")), 50);

    let start = Instant::now();
    let mut cursor = cursor_of(None);
    let mut total = 0;
    loop {
        let (rows, following) = page(&store, &cursor, 500);
        total += rows;
        match following {
            Some(next) => cursor = next,
            None => break,
        }
    }
    println!("  full pagination of {total} rows: {:?}", start.elapsed());

    let start = Instant::now();
    let mut cursor = cursor_of(Some("value number"));
    let mut total = 0;
    loop {
        let (rows, following) = page(&store, &cursor, 500);
        total += rows;
        match following {
            Some(next) => cursor = next,
            None => break,
        }
    }
    println!(
        "  full pagination with keyword of {total} rows: {:?}",
        start.elapsed()
    );

    let set_start = Instant::now();
    for i in 0..1_000 {
        store.set("import", &format!("k{i:06}"), "value").unwrap();
    }
    println!("set() x1000 (one commit each): {:?}", set_start.elapsed());

    std::fs::remove_file(path).unwrap();
}

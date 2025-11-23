use criterion::{black_box, criterion_group, criterion_main, Criterion};
use storage::PersistentStore;
use common::traits::Storage;
use tempfile::tempdir;

fn benchmark_storage_write(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("bench_db");
    let store = PersistentStore::new(db_path.to_str().unwrap()).unwrap();
    
    c.bench_function("storage_put", |b| {
        let mut i = 0u64;
        b.iter(|| {
            i += 1;
            let key = i.to_le_bytes();
            let value = [0u8; 100]; // 100 bytes value
            store.put(black_box(&key), black_box(&value)).unwrap();
        })
    });
}

criterion_group!(benches, benchmark_storage_write);
criterion_main!(benches);

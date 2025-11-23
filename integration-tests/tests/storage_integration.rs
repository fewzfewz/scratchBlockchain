use storage::PersistentStore;
use common::traits::Storage;
use tempfile::tempdir;

#[test]
fn test_storage_persistence_integration() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test_db");
    let path_str = db_path.to_str().unwrap();
    
    // Scope 1: Write data
    {
        let store = PersistentStore::new(path_str).unwrap();
        store.put(b"key1", b"value1").unwrap();
        store.put(b"key2", b"value2").unwrap();
    }
    
    // Scope 2: Read data (simulate restart)
    {
        let store = PersistentStore::new(path_str).unwrap();
        assert_eq!(store.get(b"key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(store.get(b"key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(store.get(b"key3").unwrap(), None);
    }
}

//! Block-scoped write staging.
//!
//! While a block is being applied, every write (block data, receipts,
//! accounts, EVM and WASM state, trie nodes) is held in memory. Reads see the
//! staged writes. At the end the whole block is written as one atomic batch
//! together with the updated state commitment — or discarded, leaving the
//! database exactly as it was before the block.

use crate::commitment::LtHash;
use crate::db::{BatchOp, ColumnFamily, DbError, DbMetrics, KeyValueStore, KvIter, WriteBatch};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::sync::{Arc, RwLock};

/// State keys under this prefix are node-local bookkeeping (e.g. the bridge
/// relayer's replay set) and are excluded from the state commitment.
pub const LOCAL_STATE_PREFIX: &[u8] = b"local:";

/// Meta key holding the serialized `LtHash` of the committed flat state.
pub const STATE_LTHASH_KEY: &[u8] = b"state_lthash";

type Stage = HashMap<(ColumnFamily, Vec<u8>), Option<Vec<u8>>>;

pub struct StagedStore {
    inner: Arc<dyn KeyValueStore>,
    stage: RwLock<Option<Stage>>,
}

fn poisoned() -> Box<dyn Error> {
    Box::new(DbError::new("staging lock poisoned"))
}

impl StagedStore {
    pub fn new(inner: Arc<dyn KeyValueStore>) -> Self {
        Self {
            inner,
            stage: RwLock::new(None),
        }
    }

    /// Start staging writes. Errors if a stage is already open.
    pub fn begin(&self) -> Result<(), Box<dyn Error>> {
        let mut stage = self.stage.write().map_err(|_| poisoned())?;
        if stage.is_some() {
            return Err(Box::new(DbError::new("a block stage is already open")));
        }
        *stage = Some(HashMap::new());
        Ok(())
    }

    pub fn is_staging(&self) -> bool {
        self.stage.read().map(|s| s.is_some()).unwrap_or(false)
    }

    /// Drop all staged writes.
    pub fn discard(&self) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = None;
        }
    }

    /// Commitment of the committed (on-disk) flat state.
    pub fn committed_lthash(&self) -> Result<LtHash, Box<dyn Error>> {
        Ok(self
            .inner
            .get(ColumnFamily::Meta, STATE_LTHASH_KEY)?
            .and_then(|b| LtHash::from_bytes(&b))
            .unwrap_or_default())
    }

    /// Commitment the flat state will have once the open stage is committed.
    pub fn staged_lthash(&self) -> Result<LtHash, Box<dyn Error>> {
        let mut hash = self.committed_lthash()?;
        let stage = self.stage.read().map_err(|_| poisoned())?;
        if let Some(stage) = stage.as_ref() {
            for ((cf, key), new) in stage.iter() {
                if *cf != ColumnFamily::State || key.starts_with(LOCAL_STATE_PREFIX) {
                    continue;
                }
                let old = self.inner.get(ColumnFamily::State, key)?;
                if old.as_deref() == new.as_deref() {
                    continue;
                }
                if let Some(old) = old {
                    hash.remove(key, &old);
                }
                if let Some(new) = new {
                    hash.add(key, new);
                }
            }
        }
        Ok(hash)
    }

    /// Write every staged change plus `extra` (and the updated state
    /// commitment) as one atomic batch, and close the stage.
    pub fn commit(&self, extra: WriteBatch) -> Result<LtHash, Box<dyn Error>> {
        let lthash = self.staged_lthash()?;
        let mut stage = self.stage.write().map_err(|_| poisoned())?;
        let staged = stage
            .take()
            .ok_or_else(|| Box::new(DbError::new("no block stage is open")) as Box<dyn Error>)?;

        let mut batch = WriteBatch::new();
        for ((cf, key), value) in staged {
            match value {
                Some(v) => batch.put(cf, key, v),
                None => batch.delete(cf, key),
            }
        }
        for op in extra.ops {
            match op {
                BatchOp::Put { cf, key, value } => batch.put(cf, key, value),
                BatchOp::Delete { cf, key } => batch.delete(cf, key),
            }
        }
        batch.put(ColumnFamily::Meta, STATE_LTHASH_KEY, lthash.to_bytes());
        self.inner.write_batch(batch)?;
        // One sync per block, once the whole block is on disk.
        self.inner.flush()?;
        Ok(lthash)
    }

    /// Recompute the flat-state commitment from scratch (used to initialize
    /// it, and to check it).
    pub fn recompute_lthash(&self) -> Result<LtHash, Box<dyn Error>> {
        let mut hash = LtHash::identity();
        for (key, value) in self.iter(ColumnFamily::State)? {
            if !key.starts_with(LOCAL_STATE_PREFIX) {
                hash.add(&key, &value);
            }
        }
        Ok(hash)
    }

    /// Merge staged entries for `cf` over `base` entries.
    fn merged(
        &self,
        cf: ColumnFamily,
        base: KvIter<'_>,
        in_range: impl Fn(&[u8]) -> bool,
    ) -> Result<KvIter<'static>, Box<dyn Error>> {
        let mut merged: BTreeMap<Vec<u8>, Vec<u8>> = base.collect();
        let stage = self.stage.read().map_err(|_| poisoned())?;
        if let Some(stage) = stage.as_ref() {
            for ((scf, key), value) in stage.iter() {
                if *scf != cf || !in_range(key) {
                    continue;
                }
                match value {
                    Some(v) => {
                        merged.insert(key.clone(), v.clone());
                    }
                    None => {
                        merged.remove(key);
                    }
                }
            }
        }
        Ok(Box::new(merged.into_iter()))
    }
}

impl KeyValueStore for StagedStore {
    fn get(&self, cf: ColumnFamily, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
        {
            let stage = self.stage.read().map_err(|_| poisoned())?;
            if let Some(stage) = stage.as_ref() {
                if let Some(value) = stage.get(&(cf, key.to_vec())) {
                    return Ok(value.clone());
                }
            }
        }
        self.inner.get(cf, key)
    }

    fn put(&self, cf: ColumnFamily, key: &[u8], value: &[u8]) -> Result<(), Box<dyn Error>> {
        {
            let mut stage = self.stage.write().map_err(|_| poisoned())?;
            if let Some(stage) = stage.as_mut() {
                stage.insert((cf, key.to_vec()), Some(value.to_vec()));
                return Ok(());
            }
        }
        self.inner.put(cf, key, value)
    }

    fn delete(&self, cf: ColumnFamily, key: &[u8]) -> Result<(), Box<dyn Error>> {
        {
            let mut stage = self.stage.write().map_err(|_| poisoned())?;
            if let Some(stage) = stage.as_mut() {
                stage.insert((cf, key.to_vec()), None);
                return Ok(());
            }
        }
        self.inner.delete(cf, key)
    }

    fn contains(&self, cf: ColumnFamily, key: &[u8]) -> Result<bool, Box<dyn Error>> {
        Ok(self.get(cf, key)?.is_some())
    }

    fn write_batch(&self, batch: WriteBatch) -> Result<(), Box<dyn Error>> {
        {
            let mut stage = self.stage.write().map_err(|_| poisoned())?;
            if let Some(stage) = stage.as_mut() {
                for op in batch.ops {
                    match op {
                        BatchOp::Put { cf, key, value } => {
                            stage.insert((cf, key), Some(value));
                        }
                        BatchOp::Delete { cf, key } => {
                            stage.insert((cf, key), None);
                        }
                    }
                }
                return Ok(());
            }
        }
        self.inner.write_batch(batch)
    }

    fn iter(&self, cf: ColumnFamily) -> Result<KvIter<'_>, Box<dyn Error>> {
        if !self.is_staging() {
            return self.inner.iter(cf);
        }
        self.merged(cf, self.inner.iter(cf)?, |_| true)
    }

    fn scan(
        &self,
        cf: ColumnFamily,
        start_key: &[u8],
        end_key: &[u8],
    ) -> Result<KvIter<'_>, Box<dyn Error>> {
        if !self.is_staging() {
            return self.inner.scan(cf, start_key, end_key);
        }
        let (start, end) = (start_key.to_vec(), end_key.to_vec());
        self.merged(cf, self.inner.scan(cf, start_key, end_key)?, move |k| {
            k >= start.as_slice() && k <= end.as_slice()
        })
    }

    /// While a block is staged its writes are only in memory, so there is
    /// nothing to sync: `commit` syncs once after writing the block. (Syncing
    /// here on every trie node write cost up to ~0.8 s per non-empty block,
    /// long enough to time out the next consensus round.)
    fn flush(&self) -> Result<(), Box<dyn Error>> {
        if self.is_staging() {
            return Ok(());
        }
        self.inner.flush()
    }

    fn get_metrics(&self) -> DbMetrics {
        self.inner.get_metrics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::MemDb;

    fn store() -> StagedStore {
        StagedStore::new(Arc::new(MemDb::new()))
    }

    #[test]
    fn staged_writes_are_visible_then_atomic() {
        let s = store();
        s.put(ColumnFamily::State, b"a", b"1").unwrap();
        s.begin().unwrap();
        s.put(ColumnFamily::State, b"b", b"2").unwrap();
        s.delete(ColumnFamily::State, b"a").unwrap();
        assert_eq!(
            s.get(ColumnFamily::State, b"b").unwrap(),
            Some(b"2".to_vec())
        );
        assert_eq!(s.get(ColumnFamily::State, b"a").unwrap(), None);
        let keys: Vec<Vec<u8>> = s
            .iter(ColumnFamily::State)
            .unwrap()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys, vec![b"b".to_vec()]);
        // Nothing reached the underlying store yet.
        assert_eq!(s.inner.get(ColumnFamily::State, b"b").unwrap(), None);

        s.commit(WriteBatch::new()).unwrap();
        assert_eq!(
            s.inner.get(ColumnFamily::State, b"b").unwrap(),
            Some(b"2".to_vec())
        );
        assert_eq!(s.inner.get(ColumnFamily::State, b"a").unwrap(), None);
    }

    #[test]
    fn discard_leaves_store_untouched() {
        let s = store();
        s.begin().unwrap();
        s.put(ColumnFamily::State, b"x", b"1").unwrap();
        s.discard();
        assert_eq!(s.get(ColumnFamily::State, b"x").unwrap(), None);
        assert!(!s.is_staging());
    }

    #[test]
    fn incremental_commitment_matches_recompute() {
        let s = store();
        s.begin().unwrap();
        s.put(ColumnFamily::State, b"acct1", b"100").unwrap();
        s.put(ColumnFamily::State, b"acct2", b"200").unwrap();
        s.put(ColumnFamily::State, b"local:replay", b"1").unwrap();
        s.put(ColumnFamily::Trie, b"node", b"ignored").unwrap();
        s.commit(WriteBatch::new()).unwrap();

        s.begin().unwrap();
        s.put(ColumnFamily::State, b"acct1", b"150").unwrap();
        s.delete(ColumnFamily::State, b"acct2").unwrap();
        s.put(ColumnFamily::State, b"acct3", b"5").unwrap();
        let committed = s.commit(WriteBatch::new()).unwrap();

        assert_eq!(committed, s.recompute_lthash().unwrap());
        assert_eq!(committed, s.committed_lthash().unwrap());

        let mut expected = LtHash::identity();
        expected.add(b"acct1", b"150");
        expected.add(b"acct3", b"5");
        assert_eq!(
            committed, expected,
            "local: keys and other CFs are excluded"
        );
    }
}

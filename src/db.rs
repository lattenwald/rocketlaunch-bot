use std::{collections::HashSet, path::Path};

#[derive(Debug, Clone)]
pub struct Db {
    db: sled::Db,
}

impl Db {
    pub fn open<P>(path: P) -> sled::Result<Self>
    where
        P: AsRef<Path>,
    {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn set_notified(&self, chat_id: i64, launch_id: u64) -> sled::Result<()> {
        self.db.set_merge_operator(merge_add);
        self.db.merge(
            chat_id.to_string(),
            serde_json::to_vec(&[launch_id]).unwrap_or_default(),
        )?;
        Ok(())
    }

    pub fn get_unnotified(&self, launch_id: u64) -> sled::Result<Vec<i64>> {
        let ids = self
            .db
            .iter()
            .filter_map(|a| {
                let Ok((key, val)) = a else { return None };
                let launches: HashSet<u64> = serde_json::from_slice(&val).ok()?;
                if launches.contains(&launch_id) {
                    None
                } else {
                    serde_json::from_slice::<i64>(&key).ok()
                }
            })
            .collect();
        Ok(ids)
    }

    pub fn subscribe(&self, chat_id: i64) -> sled::Result<()> {
        self.db.set_merge_operator(merge_add);
        self.db.merge(
            chat_id.to_string(),
            serde_json::to_vec::<[u64]>(&[]).unwrap(),
        )?;
        Ok(())
    }

    pub fn unsubscribe(&self, chat_id: i64) -> sled::Result<()> {
        self.db.remove(chat_id.to_string())?;
        Ok(())
    }
}

fn merge_add(_key: &[u8], old_value: Option<&[u8]>, merged_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut old: HashSet<u64> = old_value
        .map(|b| serde_json::from_slice(b).expect("bad value in db"))
        .unwrap_or_default();
    let new: HashSet<u64> = serde_json::from_slice(merged_bytes).expect("bad value");
    old.extend(new);
    Some(serde_json::to_vec(&old).expect("oops"))
}

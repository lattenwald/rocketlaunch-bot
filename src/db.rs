use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::types::Launch;

const NOTIFY_TIMES: [i64; 3] = [3600 * 24, 3600, 15 * 60];

#[derive(Debug, Clone)]
pub struct Db {
    db: sled::Db,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Notified {
    Data(i64, i64),
}

impl Db {
    pub fn open<P>(path: P) -> sled::Result<Self>
    where
        P: AsRef<Path>,
    {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn set_launches(&self, launches: &[Launch]) -> sled::Result<()> {
        self.db
            .set_merge_operator(|_key, _old, new| Some(new.to_vec()));
        self.db
            .merge("launches", serde_json::to_vec(launches).unwrap())?;
        Ok(())
    }

    pub fn get_launches(&self) -> sled::Result<Vec<Launch>> {
        let launches = self
            .db
            .get("launches")?
            .and_then(|val| serde_json::from_slice::<Vec<Launch>>(&val).ok())
            .unwrap_or_default();
        Ok(launches)
    }

    #[tracing::instrument(skip_all)]
    pub fn set_notified(
        &self,
        chat_id: i64,
        launch_id: u64,
        t0: DateTime<Utc>,
    ) -> sled::Result<()> {
        let time_diff = t0.timestamp() - Utc::now().timestamp();
        self.db.set_merge_operator(merge_add);
        self.db.merge(
            chat_id.to_string(),
            serde_json::to_vec(&HashMap::from([(launch_id, time_diff)])).unwrap_or_default(),
        )?;
        info!("set notified for {} {}", chat_id, launch_id);
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn get_unnotified(
        &self,
        launch_id: u64,
        launch_t0: DateTime<Utc>,
    ) -> sled::Result<Vec<i64>> {
        let ids = self
            .db
            .iter()
            .filter_map(|a| {
                let Ok((key, val)) = a else { return None };
                let launches: HashMap<u64, i64> = serde_json::from_slice(&val).ok()?;
                let chat_id = serde_json::from_slice::<i64>(&key).ok()?;
                let time_diff = launches.get(&launch_id).unwrap_or(&(24 * 3600));
                let until_launch = launch_t0.timestamp() - Utc::now().timestamp();
                for t in NOTIFY_TIMES {
                    if *time_diff <= t {
                        continue;
                    }
                    if until_launch <= t {
                        debug!(
                            "launch_id={} launch_t0={} t={} time_diff={} until_launch={}",
                            launch_id, launch_t0, t, time_diff, until_launch
                        );
                        return Some(chat_id);
                    }
                }
                None
            })
            .collect();
        Ok(ids)
    }

    pub fn subscribe(&self, chat_id: i64) -> sled::Result<()> {
        self.db.set_merge_operator(merge_add);
        self.db.merge(
            chat_id.to_string(),
            serde_json::to_vec::<HashMap<u64, i64>>(&HashMap::new()).unwrap(),
        )?;
        Ok(())
    }

    pub fn unsubscribe(&self, chat_id: i64) -> sled::Result<()> {
        self.db.remove(chat_id.to_string())?;
        Ok(())
    }

    pub fn iter(&self) -> sled::Iter {
        self.db.iter()
    }

    pub fn replace_chat_id(&self, old_chat_id: i64, new_chat_id: i64) -> sled::Result<bool> {
        if let Some(data) = self.db.remove(old_chat_id.to_string())? {
            let new_key = new_chat_id.to_string();
            if self.db.contains_key(&new_key)? {
                self.db.set_merge_operator(merge_add);
                self.db.merge(&new_key, data)?;
            } else {
                self.db.insert(&new_key, data)?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

fn merge_add(_key: &[u8], old_value: Option<&[u8]>, merged_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut old: HashMap<u64, i64> = old_value
        .map(|b| serde_json::from_slice(b).expect("bad value in db"))
        .unwrap_or_default();
    let new: HashMap<u64, i64> = serde_json::from_slice(merged_bytes).expect("bad value");
    old.extend(new);
    Some(serde_json::to_vec(&old).expect("oops"))
}

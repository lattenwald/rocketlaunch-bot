use std::{collections::HashMap, path::Path};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

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
                let Some(time_diff) = launches.get(&launch_id) else {
                    return Some(chat_id);
                };
                let until_launch = launch_t0.timestamp() - Utc::now().timestamp();
                for t in NOTIFY_TIMES {
                    if *time_diff <= t {
                        continue;
                    }
                    if until_launch <= t {
                        dbg!(launch_id);
                        dbg!(launch_t0);
                        dbg!(t);
                        dbg!(time_diff);
                        dbg!(until_launch);
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
}

fn merge_add(_key: &[u8], old_value: Option<&[u8]>, merged_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut old: HashMap<u64, i64> = old_value
        .map(|b| serde_json::from_slice(b).expect("bad value in db"))
        .unwrap_or_default();
    let new: HashMap<u64, i64> = serde_json::from_slice(merged_bytes).expect("bad value");
    old.extend(new);
    Some(serde_json::to_vec(&old).expect("oops"))
}

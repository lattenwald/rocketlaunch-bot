use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use rocketlaunch_bot::db::Db;

#[derive(Debug, Clone, Parser)]
struct Args {
    /// db path
    db: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let db = Db::open(args.db).expect("failed opening db");

    db.iter().for_each(|a| {
        let (k, v) = a.expect("bad data in db");
        let chat_id: i64 = serde_json::from_slice(&k).expect("bad chat_id in db");
        let notifications: HashMap<u64, i64> =
            serde_json::from_slice(&v).expect("bad notifications data in db");
        println!("{}: {:?}", chat_id, notifications);
    })
}

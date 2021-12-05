use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::GLOBAL_STORE;
use serde_json::Value;

pub trait Store {
    fn get_tiddler(&self, key: &String) -> Option<(Value, String)>;
    fn all_tiddlers(&self) -> Value;
    fn delete_tiddler(&mut self, key: &String) -> bool;
    fn put_tiddler(&mut self, key: String, meta: Value, text: String) -> u32;
    fn global_revision(&self) -> String;
    fn global_revision_num(&self) -> u64;
    fn to_json_string(&self) -> String;
}

#[derive(Serialize, Deserialize, Debug)]
struct Tiddler {
    pub meta: Value,
    pub text: String,
    pub revision: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MemoryTiddlersStore {
    tiddlers: HashMap<String, Tiddler>,
    revision: u64,
}

fn read_tiddlers_from_file<P: AsRef<Path>>(path: P) -> Result<MemoryTiddlersStore, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}

impl MemoryTiddlersStore {
    pub fn new() -> MemoryTiddlersStore {
        if let Ok(tiddlers) = read_tiddlers_from_file("./data.json") {
            tiddlers
        } else {
            MemoryTiddlersStore {
                tiddlers: Default::default(),
                revision: 0,
            }
        }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl Store for MemoryTiddlersStore {
    fn to_json_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
    fn get_tiddler(&self, key: &String) -> Option<(Value, String)> {
        if let Some(tiddler) = self.tiddlers.get(key) {
            let mut meta = tiddler.meta.clone();
            match meta {
                Value::Object(ref mut v) => {
                    v.insert("text".to_string(), Value::String(tiddler.text.clone()));
                }
                _ => {}
            }
            Some((meta, tiddler.revision.to_string()))
        } else {
            None
        }
    }

    fn all_tiddlers(&self) -> Value {
        let mut vec = Vec::new();
        for (_, tiddler) in self.tiddlers.iter() {
            vec.push(tiddler.meta.clone());
        }
        Value::Array(vec)
    }

    fn delete_tiddler(&mut self, key: &String) -> bool {
        if key != "$:%2FStoryList" || key != "$:%2FStoryList" {
            self.revision += 1;
        }
        let res = self.tiddlers.remove(key).is_some();
        actix_rt::spawn(async {
            let store = GLOBAL_STORE.read().unwrap();
            serde_json::to_writer(&File::create("./data.json").unwrap(), &*store).unwrap();
        });
        res
    }

    fn put_tiddler(&mut self, key: String, meta: Value, text: String) -> u32 {
        let rev: u32;
        if key != "$:%2FStoryList" || key != "$:%2FStoryList" {
            self.revision += 1;
        }
        if let Some(tiddler) = self.tiddlers.get_mut(&key) {
            tiddler.meta = meta;
            tiddler.text = text;
            tiddler.revision += 1;
            rev = tiddler.revision
        } else {
            self.tiddlers.insert(
                key,
                Tiddler {
                    meta,
                    text,
                    revision: 0,
                },
            );
            rev = 0
        }
        actix_rt::spawn(async {
            let store = GLOBAL_STORE.read().unwrap();
            serde_json::to_writer(&File::create("./data.json").unwrap(), &*store).unwrap();
        });
        rev
    }

    fn global_revision(&self) -> String {
        format!("{}", self.revision)
    }
    fn global_revision_num(&self) -> u64 {
        self.revision
    }
}

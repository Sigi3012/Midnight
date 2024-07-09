use once_cell::sync::OnceCell;
use serenity::{
    all::{CacheHttp, ShardMessenger},
    client::Cache,
    http::Http,
};
use std::sync::Arc;

// Adapted from
// https://github.com/scripty-bot/scripty/blob/master/scripty_bot_utils/src/extern_utils.rs

static CLIENT: OnceCell<ContextWrapper> = OnceCell::new();

pub struct ContextWrapper {
    pub shard: ShardMessenger,
    pub cache: Arc<Cache>,
    pub http: Arc<Http>,
}

impl CacheHttp for ContextWrapper {
    fn http(&self) -> &Http {
        &self.http
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.cache)
    }
}

pub fn get_context_wrapper() -> &'static ContextWrapper {
    CLIENT
        .get()
        .expect("Set context wrapper before calling get")
}

pub fn set_context_wrapper(shard: ShardMessenger, http: Arc<Http>, cache: Arc<Cache>) {
    let _ = CLIENT.set(ContextWrapper { shard, cache, http });
}

use crate::features::{groups::GroupManager, mapfeed::MapfeedManager};
use std::sync::OnceLock;
use tracing::{info, warn};

static INITIALIZED: OnceLock<()> = OnceLock::new();

pub async fn init_tasks() {
    if INITIALIZED.get().is_some() {
        warn!("Tasks have already been initialised");
        return;
    } else {
        INITIALIZED
            .set(())
            .expect("Failed to set background task status to initialised")
    }

    MapfeedManager::new();
    GroupManager::new();

    info!("Initialized task manager");
}

use backend::{
    api::osu::AuthenticationManager,
    groups::GroupManager,
    mapfeed::{MapfeedManager, populate},
};
use log::{info, warn};
use once_cell::sync::OnceCell;
use tokio::time::{Duration, sleep};

static INITIALIZED: OnceCell<()> = OnceCell::new();
const THREE_SECONDS_DURATION: Duration = Duration::new(3, 0);

pub async fn init_tasks() {
    if INITIALIZED.get().is_some() {
        warn!("Tasks have already been initialised");
        return;
    } else {
        INITIALIZED
            .set(())
            .expect("Failed to set background task status to initialised")
    }

    // TODO Ability to manage if the loop is running or not
    AuthenticationManager::new().await;

    // Sleep for a little to prevent accessing api before authentication returns
    sleep(THREE_SECONDS_DURATION).await;
    match populate().await {
        Ok(_) => (),
        Err(e) => {
            panic!("Error while populating database, error: {}", e);
        }
    }

    MapfeedManager::start().await;
    GroupManager::new();

    info!("Initialized task manager");
}

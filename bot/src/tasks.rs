use backend::{
    api::osu::AuthenticationManager,
    mapfeed::{populate, MapfeedManager},
};
use log::{info, warn};
use once_cell::sync::OnceCell;
use tokio::time::{sleep, Duration};

static INITALIZED: OnceCell<()> = OnceCell::new();
const THREE_SECONDS_DURATION: Duration = Duration::new(3, 0);

pub async fn init_tasks() {
    if INITALIZED.get().is_some() {
        warn!("Tasks have already been initalized");
        return;
    } else {
        INITALIZED
            .set(())
            .expect("Failed to set background task status to initalized")
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
    info!("Initalized task manager");
}

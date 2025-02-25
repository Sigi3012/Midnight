use anyhow::{Result, bail};
use diesel_async::{
    AsyncPgConnection,
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        AsyncDieselConnectionManager,
        bb8::{Pool, PooledConnection},
    },
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use std::time::Duration;
use tokio::{task, time};
use tracing::warn;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations");

pub type DbPool = Pool<AsyncPgConnection>;
pub type PooledConn<'a> = PooledConnection<'a, AsyncPgConnection>;

#[derive(Debug)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    pub async fn new(db_url: &str) -> Result<Self> {
        let mut pool = Pool::builder()
            //.max_size(1)
            .max_lifetime(Some(Duration::new(45 * 60, 0)))
            .build(AsyncDieselConnectionManager::<AsyncPgConnection>::new(
                db_url,
            ))
            .await
            .expect("Database pool should be constructable");

        if let Err(why) = run_migrations(&mut pool).await {
            bail!(
                "Fatal! Something went wrong initialising the database, {}",
                why
            );
        }

        Ok(Self { pool })
    }

    // Not sure how I feel about this one
    pub async fn get_conn(&self) -> PooledConn {
        for i in 0..5 {
            match self.pool.get().await {
                Ok(p) => return p,
                Err(e) => {
                    warn!("{e}. Attempt {i}/3");
                    time::sleep(Duration::from_millis(500 * i)).await;
                }
            }
        }
        panic!("Cannot get connection from pool.");
    }
}

async fn run_migrations(
    pool: &mut DbPool,
) -> Result<(), Box<dyn std::error::Error + Sync + Send + 'static>> {
    let conn = pool.dedicated_connection().await?;
    let mut wrapper: AsyncConnectionWrapper<AsyncPgConnection> =
        diesel_async::async_connection_wrapper::AsyncConnectionWrapper::from(conn);

    task::spawn_blocking(move || {
        let _ = wrapper.run_pending_migrations(MIGRATIONS);
    })
    .await?;

    Ok(())
}

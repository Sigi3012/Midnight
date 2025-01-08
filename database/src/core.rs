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
use std::{env, sync::OnceLock, time::Duration};
use tokio::{task, time};
use tracing::warn;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../migrations");

pub type DbPool = Pool<AsyncPgConnection>;
pub type DbPooledConnection<'a> = PooledConnection<'a, AsyncPgConnection>;

static INITIALISED: OnceLock<()> = OnceLock::new();
pub static DB: OnceLock<Database> = OnceLock::new();
/*
pub enum Connection<'a> {
    PooledConnection(PooledConnection<'a, AsyncPgConnection>),
    Dedicated(AsyncPgConnection),
}

 */

pub(crate) mod macros {
    macro_rules! get_conn {
        () => {
            &mut DB.get().unwrap().get_conn().await
        };
    }
    pub(crate) use get_conn;
}

#[derive(Debug)]
pub struct Database {
    pool: DbPool,
}

impl Database {
    async fn new() -> Result<Self> {
        if INITIALISED.get().is_some() {
            bail!("Database pool already initialised");
        } else {
            INITIALISED
                .set(())
                .expect("This should never try to write to `INITIALISED` due to previous check")
        }

        let mut pool = Pool::builder()
            .max_lifetime(Some(Duration::new(45 * 60, 0)))
            .build(AsyncDieselConnectionManager::<AsyncPgConnection>::new(
                env::var("DATABASE_URL").expect("DATABASE_URL should be set in .env"),
            ))
            .await
            .expect("Database pool should be constructable");

        match run_migrations(&mut pool).await {
            Ok(_) => Ok(Self { pool }),
            Err(e) => {
                bail!(
                    "Fatal! Something went wrong initialising the database, {}",
                    e
                );
            }
        }
    }

    pub async fn get_conn(&self) -> DbPooledConnection {
        for i in 0..3 {
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

    pub async fn get_conn_unchecked(&self) -> DbPooledConnection {
        self.pool
            .get()
            .await
            .expect("Should be able to get connection from pool.")
    }
}

pub async fn initialise() -> Result<()> {
    let _ = DB.set(Database::new().await?);
    Ok(())
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

use crate::DB_CONFIG;
use deadpool_postgres::{Client, PoolError, RecyclingMethod};
use deadpool_postgres::{Manager, ManagerConfig, Pool};
use log::{error, info};
use once_cell::sync::OnceCell;
use std::ops::DerefMut;
use thiserror::Error;
use tokio_postgres::NoTls;

const DATABASE_CREATION_QUERY: &str = r#"
    CREATE DATABASE midnight OWNER postgres;
"#;

static INITALIZED: OnceCell<()> = OnceCell::new();
pub static DB_POOL: tokio::sync::OnceCell<Pool> = tokio::sync::OnceCell::const_new();

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("../migrations");
}

#[derive(Debug, Error)]
pub enum CreateTablesError {
    #[error("Postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    #[error("Tables have already been created")]
    AlreadyInitalized(),
}

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Postgres error: {0}")]
    TokioPostgres(#[from] tokio_postgres::Error),
    #[error("Connection pool error: {0}")]
    PoolError(#[from] PoolError),
    #[error("Unexpected result from database")]
    UnexpectedResult,
}

async fn check_database_existance() -> Result<(), tokio_postgres::Error> {
    match DB_CONFIG.connect(NoTls).await {
        Ok(_) => Ok(()),
        Err(_) => {
            let mut changed_config = DB_CONFIG.clone();
            changed_config.dbname("postgres");

            let (client, connecton) = changed_config.connect(NoTls).await?;

            tokio::spawn(async move {
                if let Err(why) = connecton.await {
                    error!("Connection error: {}", why)
                }
            });
            info!("Database does not exist, attempting to create");
            client.execute(DATABASE_CREATION_QUERY, &[]).await?;

            info!("Successfully created database");
            Ok(())
        }
    }
}

async fn create_connection_pool() -> Pool {
    let pg_config = DB_CONFIG.clone();
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
    let pool = Pool::builder(mgr).max_size(16).build().unwrap();

    pool
}

pub async fn get_client_from_pool() -> Result<Client, PoolError> {
    let client = DB_POOL.get().unwrap().get().await?;
    Ok(client)
}

async fn run_migrations() {
    let pool = DB_POOL.get().unwrap();
    let mut conn = pool.get().await.unwrap();
    let client = conn.deref_mut().deref_mut();
    let report = embedded::migrations::runner().run_async(client).await;

    match report {
        Ok(_) => info!("Successfully ran migrations"),
        Err(e) => {
            panic!("Database migrations error: {}", e)
        }
    }
}

pub async fn initialize_database() -> Result<(), CreateTablesError> {
    if INITALIZED.get().is_some() {
    } else {
        INITALIZED
            .set(())
            .expect("Failed to set background task status to initalized")
    }

    check_database_existance().await?;
    let pool = create_connection_pool().await;
    let _ = DB_POOL.set(pool);
    run_migrations().await;

    Ok(())
}

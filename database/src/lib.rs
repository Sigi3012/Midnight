use lazy_static::lazy_static;
use std::env;
use tokio_postgres::Config;

lazy_static! {
    pub static ref DB_CONFIG: Config = {
        let mut conf = Config::new();
        conf.user(&env::var("POSTGRES_USERNAME").unwrap());
        conf.password(&env::var("POSTGRES_PASSWORD").unwrap());
        conf.dbname("midnight");
        conf.host(&env::var("POSTGRES_HOST").unwrap_or("localhost".to_string()));
        conf.port(
            env::var("POSTGRES_PORT")
                .unwrap_or("5432".to_string())
                .parse()
                .unwrap(),
        );
        conf
    };
}

pub mod core;
pub mod mapfeed;
pub mod subscriptions;

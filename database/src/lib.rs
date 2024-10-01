use lazy_static::lazy_static;
use std::env;
use tokio_postgres::Config;

lazy_static! {
    pub static ref DB_CONFIG: Config = {
        let mut conf = Config::new();
        conf.user(
            &env::var("POSTGRES_USERNAME")
                .expect("POSTGRES_USERNAME should be set in .env or .docker-compose.yml"),
        );
        conf.password(
            &env::var("POSTGRES_PASSWORD")
                .expect("POSTGRES_PASSWORD should be set in .env or .docker-compose.yml"),
        );
        conf.dbname("midnight");
        conf.host(&env::var("POSTGRES_HOST").unwrap_or("localhost".to_string()));
        conf.port(
            env::var("POSTGRES_PORT")
                .unwrap_or("5432".to_string())
                .parse()
                .expect("Port should be parsable"),
        );
        conf
    };
}

pub mod core;
pub mod mapfeed;
pub mod subscriptions;

pub mod models;
pub mod schema;

use std::env;

use dotenvy::dotenv;
use diesel::{Connection, PgConnection};

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL")
        .expect("Environment variable 'DATABASE_URL' must be set");
    PgConnection::establish(&db_url).unwrap()
}

pub mod competition;
pub mod game;
pub mod variant;
pub mod result;
pub mod series;

pub type UtcDateTime = chrono::DateTime<chrono::offset::Utc>;
pub type Date = chrono::NaiveDate;
pub type Tx = sqlx::Transaction<sqlx::pool::PoolConnection<sqlx::PgConnection>>;

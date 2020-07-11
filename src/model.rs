pub mod competition;
pub mod game;
pub mod variant;

use serde::{Serialize, Deserialize};
use actix_web::{HttpResponse, HttpRequest, Responder, Error, web};
use futures::future::{ready, Ready};
use sqlx::{PgPool, FromRow, Row};
use sqlx::postgres::PgRow;
use anyhow::Result;
use sqlx::postgres::*;

pub type UtcDateTime = chrono::DateTime<chrono::offset::Utc>;
pub type Date = chrono::NaiveDate;
pub type Tx = sqlx::Transaction<sqlx::pool::PoolConnection<sqlx::PgConnection>>;
// %Y-%m-%dT%H:%M:%S.%f%z
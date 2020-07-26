use serde::{Serialize, Deserialize};
use crate::{DbViewerPool, DbAdminPool, model::{Tx, UtcDateTime}};
use anyhow::Result;

#[derive(Deserialize)]
pub struct Series {
    name: String,
    first_n: i16,
    top_n: i16,
}

pub async fn add_series(
    pool: &DbAdminPool,
    series: Vec<Series>,
) -> Result<()> {
    // if a single competition causes an error, don't commit any
    let mut tx = pool.0.begin().await?;
    for series in series {
        // I'd prefer to use references throughout, but I don't know a better pattern that would
        // allow me to pass the same mutable borrow to multiple functions.
        tx = add_single_series(tx, series).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn get_series_names(
    pool: &DbViewerPool,
) -> Result<Vec<String>> {
    let series_name_records = sqlx::query!(
        "select name
        from series
        order by name desc",
    ).fetch_all(&pool.0).await?;
    let series_names = series_name_records.into_iter().map(
        |record| record.name).collect();
    Ok(series_names)
}


async fn add_single_series(
    mut tx: Tx,
    series: Series,
) -> Result<Tx> {
    sqlx::query!(
        "INSERT INTO series (
            name
          , first_n
          , top_n
        ) VALUES (
            $1
          , $2
          , $3
        )",
        series.name,
        series.first_n,
        series.top_n,
    ).execute(&mut tx).await?;
    Ok(tx)
}

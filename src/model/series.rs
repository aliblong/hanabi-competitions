use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::{DbViewerPool, DbAdminPool, model::{Tx, UtcDateTime}};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct CompetitionResultRecordSummary {
    pub player_name: String,
    pub frac_mp: f64,
}

#[derive(Serialize, Deserialize)]
pub struct LeaderboardRecord {
    pub player_name: String,
    pub sum_mp: f64,
    pub mean_mp: f64,
    pub competition_results: Vec<CompetitionResultRecordSummary>
}

pub async fn get_series_leaderboard(
    pool: &DbViewerPool,
    series_name: &str,
) -> Result<Vec<LeaderboardRecord>> {
    let leaderboard_records = sqlx::query!(
        "select
            player_name
          , competition_name
          , fractional_mp
        from series_leaderboards
        where series_name = $1
        --order by player_name",
        series_name,
    ).fetch_all(&pool.0).await?;
    let mut leaderboard_games = HashMap::new();
    for record in leaderboard_records.into_iter() {
        let player_name = record.player_name.unwrap();
        let competition_name = record.competition_name.unwrap();
        let frac_mp = record.fractional_mp.unwrap();
        match leaderboard_games.get_mut(&player_name) {
            None => {
                leaderboard_games.insert(
                    player_name,
                    (None, vec![(competition_name, frac_mp)])
                );
            }
            Some((_, competitions)) => {
                competitions.push((competition_name, frac_mp));
            }
        }
    }
    let leaderboard_aggregate_records = sqlx::query!(
        "select
            player_name
          , sum(fractional_mp) sum_frac_mp
          , avg(fractional_mp) mean_frac_mp
        from series_leaderboards
        where series_name = $1
        group by player_name
        --order by player_name",
        series_name,
    ).fetch_all(&pool.0).await?;
    for record in leaderboard_aggregate_records.into_iter() {
        let (aggregate_records, _) = leaderboard_games.get_mut(&record.player_name.unwrap()).unwrap();
        *aggregate_records = Some((record.sum_frac_mp.unwrap(), record.mean_frac_mp.unwrap()));
    }
    Ok(leaderboard_games.into_iter().map(|(player, record)| {
        LeaderboardRecord {
            player_name: player,
            sum_mp: record.0.unwrap().0,
            mean_mp: record.0.unwrap().1,
            competition_results: record.1.into_iter().map(|result|
                CompetitionResultRecordSummary {
                    player_name: result.0,
                    frac_mp: result.1,
                }
            ).collect(),
        }
    }).collect())
}

pub async fn get_series_competitions(
    pool: &DbViewerPool,
    series_name: &str,
) -> Result<Vec<String>> {
    let competitions_name_records = sqlx::query!(
        "select competition_names.name
        from series
        join series_competitions on series.id = series_id
        join competition_names using(competition_id)
        where series.name = $1
        order by name desc",
        series_name,
    ).fetch_all(&pool.0).await?;
    let series_names = competitions_name_records.into_iter().map(
        |record| record.name.unwrap()).collect();
    Ok(series_names)
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

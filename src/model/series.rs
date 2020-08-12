use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::{
    DbViewerPool,
    DbAdminPool,
    model::{
        Tx,
        //UtcDateTime,
        competition::{
            get_competition_with_ids,
            competition_with_derived_quantities_from_ruleset_with_ids, 
            CompetitionWithDerivedQuantities,
        }
    }
};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct Series {
    name: String,
    first_n: Option<i16>,
    top_n: Option<i16>,
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
    pub competition_name: String,
    pub frac_mp: f64,
}

#[derive(Serialize, Deserialize)]
pub struct LeaderboardRecord {
    pub player_name: String,
    pub score: f64,
    pub mean_frac_mp: f64,
    pub competition_results: Vec<Option<CompetitionResultRecordSummary>>,
}

#[derive(Serialize, Deserialize)]
pub struct SeriesView {
    series_name: String,
    active_competitions: Vec<CompetitionWithDerivedQuantities>,
    past_competition_names: Vec<String>,
    leaderboard_records: Vec<LeaderboardRecord>,
    competition_scores_table_headers: Vec<String>,
}

async fn verify_series_exists(
    pool: &DbViewerPool,
    series_name: &str,
) -> Result<bool> {
    match sqlx::query!(
        "select count(*) cnt
        from series
        where name = $1",
        series_name,
    ).fetch_one(&pool.0).await?.cnt.unwrap() {
        1 => Ok(true),
        0 => Ok(false),
        // series name is logically unique
        _ => unreachable!(),
    }
}

pub async fn get_series_view(
    pool: &DbViewerPool,
    series_name: &str,
    max_num_comps: i64,
) -> Result<SeriesView> {
    if !verify_series_exists(pool, series_name).await? {
        return Err(GetSeriesError::NotFound.into());
    }
    let (leaderboard_records, num_comps) =
        get_series_leaderboard(pool, series_name, max_num_comps).await?;
    Ok(SeriesView {
        series_name: series_name.to_owned(),
        active_competitions: get_series_active_competitions(pool, series_name).await?,
        past_competition_names: get_series_past_competition_names(pool, series_name).await?,
        leaderboard_records,
        competition_scores_table_headers:
            (1..num_comps + 1).map(|i| format!("comp {}", i)).collect(),
    })
}

#[derive(thiserror::Error, Debug)]
pub enum GetSeriesError {
    #[error("No series with that name was found")]
    NotFound,
}


async fn get_series_leaderboard(
    pool: &DbViewerPool,
    series_name: &str,
    max_num_comps: i64,
) -> Result<(Vec<LeaderboardRecord>, i64)> {
    let leaderboard_aggregate_records = sqlx::query!(
        "select
            player_name
          , score
          , mean_frac_mp
        from series_player_scores
        where series_name = $1",
        series_name,
    ).fetch_all(&pool.0).await?;
    if !series_name.starts_with("All-time") && leaderboard_aggregate_records.len() != 0 {
        let num_comps = sqlx::query!(
            "select max(num_comps) max_num_comps
            from (
                select count(*) num_comps
                from series_competition_results
                where series_name = $1
                group by player_name
            ) _",
            series_name,
        ).fetch_one(&pool.0).await?.max_num_comps.unwrap();
        if num_comps <= max_num_comps {
            let mut leaderboard_games = HashMap::new();
            for record in leaderboard_aggregate_records.into_iter() {
                let player_name = record.player_name.unwrap();
                leaderboard_games.insert(
                    player_name,
                    (Some((record.score.unwrap(), record.mean_frac_mp.unwrap())), vec![])
                );
            }
            let leaderboard_records = sqlx::query!(
                "select
                    player_name
                  , competition_name
                  , fractional_mp
                from series_competition_results
                where series_name = $1",
                series_name,
            ).fetch_all(&pool.0).await?;
            for record in leaderboard_records.into_iter() {
                let (_, competitions) = leaderboard_games.get_mut(&record.player_name.unwrap()).unwrap();
                competitions.push((record.competition_name.unwrap(), record.fractional_mp.unwrap()))
            }
            let mut records = leaderboard_games.into_iter().map(|(player, record)| {
                let mut competition_results: Vec<Option<CompetitionResultRecordSummary>> =
                    record.1.into_iter().map(|result|
                        Some(CompetitionResultRecordSummary {
                            competition_name: result.0,
                            frac_mp: result.1,
                        })
                    ).collect();
                competition_results.extend(
                    (competition_results.len()..num_comps as usize).map(|_| None)
                );
                LeaderboardRecord {
                    player_name: player,
                    score: record.0.unwrap().0,
                    mean_frac_mp: record.0.unwrap().1,
                    competition_results,
                }
            }).collect::<Vec<LeaderboardRecord>>();
            records.sort_unstable_by(|r1, r2| r2.score.partial_cmp(&r1.score).unwrap());
            return Ok((
                records,
                num_comps,
            ));
        }
    }
    let mut records = leaderboard_aggregate_records.into_iter().map(|record| {
        LeaderboardRecord {
            player_name: record.player_name.unwrap(),
            score: record.score.unwrap(),
            mean_frac_mp: record.mean_frac_mp.unwrap(),
            competition_results: vec![],
        }
    }).collect::<Vec<LeaderboardRecord>>();
    records.sort_unstable_by(|r1, r2| r2.score.partial_cmp(&r1.score).unwrap());
    Ok((records, 0))
}

async fn get_series_active_competitions(
    pool: &DbViewerPool,
    series_name: &str,
) -> Result<Vec<CompetitionWithDerivedQuantities>> {
    let series_active_competition_id_records = sqlx::query!(
        r#"select competitions.id
        from series
        join series_competitions on series.id = series_id
        join competition_names using(competition_id)
        join competitions on competition_id = competitions.id
        where series.name = $1
            and end_datetime > now()
        order by competition_names.name desc"#,
        series_name,
    ).fetch_all(&pool.0).await?;
    let series_active_competition_ids: Vec<i16> = series_active_competition_id_records.into_iter().map(
        |record| record.id).collect();
    let mut active_competitions_rulesets_with_ids = Vec::new();
    for id in series_active_competition_ids.into_iter() {
        active_competitions_rulesets_with_ids.push(get_competition_with_ids(pool, id).await?);
    }
    let mut competitions_with_derived_quantities = Vec::new();
    for ruleset_with_ids in active_competitions_rulesets_with_ids {
        competitions_with_derived_quantities.push(
            competition_with_derived_quantities_from_ruleset_with_ids(
                pool,
                ruleset_with_ids,
            ).await?
        );
    }
    Ok(competitions_with_derived_quantities)
}

async fn get_series_past_competition_names(
    pool: &DbViewerPool,
    series_name: &str,
) -> Result<Vec<String>> {
    let competitions_name_records = sqlx::query!(
        r#"select competition_names.name
        from series
        join series_competitions on series.id = series_id
        join competition_names using(competition_id)
        join competitions on competition_id = competitions.id
        where series.name = $1
            and end_datetime < now()
        order by name desc"#,
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

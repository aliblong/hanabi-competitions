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

#[derive(Serialize, Deserialize)]
pub struct SeriesView {
    active_competitions: Vec<CompetitionWithDerivedQuantities>,
    past_competition_names: Vec<String>,
    leaderboard_records: Vec<LeaderboardRecord>,
    competition_scores_table_headers: Vec<String>,
}

pub async fn get_series_view(
    pool: &DbViewerPool,
    series_name: &str,
    max_num_comps: i64,
) -> Result<SeriesView> {
    let (leaderboard_records, num_comps) =
        get_series_leaderboard(pool, series_name, max_num_comps).await?;
    Ok(SeriesView {
        active_competitions: get_series_active_competitions(pool, series_name).await?,
        past_competition_names: get_series_past_competition_names(pool, series_name).await?,
        leaderboard_records,
        competition_scores_table_headers:
            (1..num_comps + 1).map(|i| format!("comp {}", i)).collect(),
    })
}

async fn get_series_leaderboard(
    pool: &DbViewerPool,
    series_name: &str,
    max_num_comps: i64,
) -> Result<(Vec<LeaderboardRecord>, i64)> {
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
    let mut leaderboard_games = HashMap::new();
    for record in leaderboard_aggregate_records.into_iter() {
        let player_name = record.player_name.unwrap();
        leaderboard_games.insert(
            player_name,
            (Some((record.sum_frac_mp.unwrap(), record.mean_frac_mp.unwrap())), vec![])
        );
    }
    let num_comps = sqlx::query!(
        "select max(num_comps) max_num_comps
        from (
            select count(*) num_comps
            from series_leaderboards
            group by player_name
        ) _"
    ).fetch_one(&pool.0).await?.max_num_comps.unwrap();
    println!("{}", num_comps);
    if num_comps <= max_num_comps {
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
        for record in leaderboard_records.into_iter() {
            let (_, competitions) = leaderboard_games.get_mut(&record.player_name.unwrap()).unwrap();
            competitions.push((record.competition_name.unwrap(), record.fractional_mp.unwrap()))
        }
    }
    Ok((
        leaderboard_games.into_iter().map(|(player, record)| {
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
        }).collect(),
        num_comps,
    ))
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
            and end_datetime > date('2020-06-01')
        order by competition_names.name desc"#, //now()",
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
            and end_datetime < date('2020-06-01')
        order by name desc"#, //now()",
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

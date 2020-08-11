use sqlx::postgres::*;
use crate::model::UtcDateTime;
use anyhow::Result;

// This is quite similar to model::competition::CompetitionFlatResult, but this one is designed
// to be a raw, complete view, for analysis, where the other is tailored to be a good
// default view of the results, intended to be nested.
#[derive(sqlx::FromRow, serde::Serialize)]
pub struct CombinedResult {
    pub competition_name: String,
    pub final_rank: i64,
    pub fractional_mp: f64,
    pub sum_mp: i64,
    pub player_name: String,
    pub base_seed_name: String,
    pub seed_matchpoints: i32,
    pub site_game_id: i64,
    pub replay_url: String,
    pub score: i16,
    pub turns: i16,
    pub datetime_game_started: UtcDateTime,
    pub datetime_game_ended: UtcDateTime,
    pub character_name: Option<String>,
}

pub async fn get_combined_results(
    pool: &crate::DbViewerPool,
    query_where_clause: &Option<String>,
) -> Result<Vec<CombinedResult>> {
    let clause_to_insert = {
        if let Some(clause) = query_where_clause {
            format!("where {}", clause)
        } else {
            "".to_owned()
        }
    };
    let result = sqlx::query_as::<sqlx::Postgres, CombinedResult>(&format!(r#"
            -- intentional sql injection, so make sure account doesn't have any more
            -- privileges than select
            select *
            from computed_competition_standings
            {}
            order by
                competition_name desc
              , sum_mp desc
              , base_seed_name
              , replay_url
              , player_name
        "#, clause_to_insert))
        .fetch_all(&pool.0)
        .await?;
    Ok(result)
}

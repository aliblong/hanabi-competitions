use sqlx::postgres::*;
use crate::model::UtcDateTime;
use anyhow::Result;

pub async fn get_arbitrary_competitions_results(
    pool: &crate::DbViewerPool,
    query_where_clause: &Option<String>,
) -> Result<Vec<CompetitionResult>> {
    let clause_to_insert = {
        if let Some(clause) = query_where_clause {
            format!("where {}", clause)
        } else {
            "".to_owned()
        }
    };
    let result = sqlx::query_as::<sqlx::Postgres, CompetitionResult>(&format!(r#"
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

pub async fn flat_results_to_json(results: Vec<CompetitionResult>) -> Result<String> {
    let jsonified_results = serde_json::to_string(&results)?;
    Ok(jsonified_results)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct CompetitionResult {
    pub competition_name: String,
    pub final_rank: i64,
    pub fractional_mp: f64,
    pub sum_mp: i64,
    pub player_name: String,
    pub base_seed_name: String,
    pub seed_matchpoints: i32,
    pub replay_url: String,
    pub score: i16,
    pub turns: i16,
    pub datetime_game_started: UtcDateTime,
    pub datetime_game_ended: UtcDateTime,
    pub character_name: Option<String>,
}

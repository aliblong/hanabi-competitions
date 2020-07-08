use std::collections::HashMap;
use serde::Serialize;
use sqlx::postgres::*;
use crate::model::UtcDateTime;
use anyhow::Result;

pub async fn get_single_competition_results(
    pool: &crate::DbViewerPool,
    competition_name: &str,
) -> Result<(Vec<SingleCompetitionResult>, i16)> {
    let results = sqlx::query_as!(
        SingleCompetitionResult,
        "select
            final_rank
          , fractional_MP
          , sum_MP
          , player_name
          , base_seed_name
          , seed_matchpoints
          , replay_URL
          , site_game_id
          , score
          , turns
          , cast(round(extract(epoch from (datetime_game_ended - datetime_game_started))) as int)
                as game_duration_seconds
        from computed_competition_standings
        where competition_name = $1",
        competition_name,
    ).fetch_all(&pool.0).await?;
    let team_size = sqlx::query!(
        "select
            num_players
        from competition_names
        join competitions using(id)
        where name = $1",
        competition_name
    ).fetch_one(&pool.0).await?.num_players;
    Ok((results, team_size))
}

pub fn nested_results_from_single_competition_results(
    flat_results: Vec<SingleCompetitionResult>,
    team_size: i16,
) -> CompetitionNestedResults {
    // Self {
    let mut base_seed_names: Vec<String> = flat_results.iter().map(|r| r.base_seed_name.clone())
        .collect();
    base_seed_names.sort_unstable();
    base_seed_names.dedup();
    let mut player_indexed_results = HashMap::new(); // ::<String, (Vec<i64>, Vec<SingleCompetitionResult>)>
    for result in flat_results.into_iter() {
        match player_indexed_results.get_mut(&result.player_name) {
            None => {
                player_indexed_results.insert(
                    result.player_name.clone(),
                    (
                        vec![result.site_game_id],
                        vec![Some(result)]
                    )
                );
            },
            Some((games, results)) => {
                games.push(result.site_game_id);
                results.push(Some(result));
            },
        }
    }
    let mut game_combination_indexed_results = HashMap::new(); // ::<String, TeamResults>
    for (player_name, (mut games, results)) in player_indexed_results.into_iter() {
        games.sort_unstable();
        let games_key = games.into_iter().map(|site_game_id| site_game_id.to_string())
            .collect::<Vec<String>>().join("-");
        match game_combination_indexed_results.get_mut(&games_key) {
            None => {
                game_combination_indexed_results.insert(
                    games_key,
                    (vec![Some(player_name)], results)
                );
            }
            Some((players, _)) => {
                players.push(Some(player_name));
            }
        }
    }
    let mut competition_nested_results = CompetitionNestedResults{
        base_seed_names,
        team_size,
        team_results: Vec::new(),
    };
    for (_, (mut players, mut results)) in game_combination_indexed_results.into_iter() {
        players.sort_unstable();
        while players.len() < team_size as usize {
            players.push(None);
        }
        //results.sort_unstable_by_key(|r| r.as_ref().unwrap().base_seed_name.clone());
        results.sort_unstable_by(|r1,  r2|
            r1.as_ref().unwrap().base_seed_name.cmp(&r2.as_ref().unwrap().base_seed_name));
        //results.sort_unstable_by(|r| match r {
        //    Some(a) => a.base_seed_name,
        //    None => unreachable!(),
        //});
        for (idx, base_seed_name) in competition_nested_results.base_seed_names.iter().enumerate() {
            if results.iter().find(
                |&r| r.is_some() && &r.as_ref().unwrap().base_seed_name == base_seed_name
            ).is_none() {
                results.insert(idx, None);
            }
        }
        let first_result = results.iter().find(|&r| r.is_some()).unwrap().as_ref().unwrap();
        competition_nested_results.team_results.push(TeamResults{
            players,
            final_rank: first_result.final_rank, 
            fractional_mp: first_result.fractional_mp, 
            sum_mp: first_result.sum_mp, 
            game_results: results.into_iter().map(|optional_result| match optional_result {
                Some(result) => Some(GameResult{
                    seed_matchpoints: result.seed_matchpoints,
                    score: result.score,
                    turns: result.turns,
                    site_game_id: result.site_game_id,
                    replay_url: result.replay_url,
                    game_duration_seconds: result.game_duration_seconds,
                }),
                None => None,
            }).collect(),
        })
    }
    competition_nested_results
}

    // let mut games: Vec<i64> = flat_results.iter().map(|r| r.site_game_id).collect();
    // seeds.sort_unstable();
    // seeds.dedup();

#[derive(Serialize, Debug)]
pub struct CompetitionNestedResults {
    pub base_seed_names: Vec<String>,
    pub team_size: i16,
    pub team_results: Vec<TeamResults>,
}

#[derive(Serialize, Debug)]
pub struct TeamResults {
    pub players: Vec<Option<String>>,
    pub final_rank: i64,
    pub fractional_mp: f64,
    pub sum_mp: i64,
    pub game_results: Vec<Option<GameResult>>,
}

#[derive(Serialize, Debug)]
pub struct GameResult {
    pub seed_matchpoints: i32,
    pub score: i16,
    pub turns: i16,
    pub site_game_id: i64,
    pub replay_url: String,
    pub game_duration_seconds: i32,
}

pub fn flat_results_to_json(results: Vec<SingleCompetitionResult>) -> Result<String> {
    let jsonified_results = serde_json::to_string(&results)?;
    Ok(jsonified_results)
}

#[derive(sqlx::FromRow, Serialize, Clone, Debug)]
pub struct SingleCompetitionResult {
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
    // support for the INTERVAL type landed literally less than a week ago, so look out for a
    // release: https://github.com/launchbadge/sqlx/pull/271
    pub game_duration_seconds: i32,
}

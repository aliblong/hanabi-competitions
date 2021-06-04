use anyhow::Result;
use serde::{Serialize, Deserialize};
use crate::model::{UtcDateTime, Date, Tx};

#[derive(thiserror::Error, Debug)]
pub enum SeedGamesError {
    #[error("Consistency error in competition results: {0}")]
    Consistency(String),
}

impl SeedGames {
    pub fn validate(&self) -> Result<(), SeedGamesError> {
        let num_players = self.num_players;
        for game_results in &self.games {
            if game_results.players.len() != num_players as usize
            {
                return Err(SeedGamesError::Consistency(format!(
                    "num_players = {}; players = {:?}", num_players, &game_results.players
                )));
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct SeedGames {
    pub num_players: i16,
    pub variant_id: i32,
    pub base_seed_name: String,
    pub games: Vec<Game>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Game {
    pub players: Vec<String>,
    pub game_id: i64,
    pub score: i16,
    pub turns: i16,
    pub datetime_started: UtcDateTime,
    pub datetime_ended: UtcDateTime,
}

pub async fn add_seeds_games(
    pool: &super::super::DbAdminPool,
    seeds_games: &Vec<SeedGames>,
) -> Result<()> {
    // if a single competition causes an error, don't commit any
    let mut tx = pool.0.begin().await?;
    for seed_games in seeds_games {
        // I'd prefer to use references throughout, but I don't know a better pattern that would
        // allow me to pass the same mutable borrow to multiple functions.
        tx = add_seed_games(tx, seed_games).await?;
    }
    sqlx::query("select update_computed_competition_standings()").execute(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn upsert_seed(
    mut tx: Tx,
    num_players: i16,
    variant_id: i32,
    base_seed_name: &str,
) -> Result<(Tx, i16)> {
    let seed_id = sqlx::query!(
        "WITH new_seed AS(
            INSERT INTO seeds (
                num_players
              , variant_id
              , base_name
            ) VALUES (
                $1
              , $2
              , $3
            )
            ON CONFLICT DO NOTHING
            RETURNING id
        )
        SELECT COALESCE(
            (select id from new_seed)
          , (select seeds.id
                FROM seeds
                JOIN variants on variant_id = variants.id
                WHERE
                    num_players = $1
                    and variants.site_variant_id = $2
                    and base_name = $3
            )
        ) id
        ",
        num_players,
        variant_id,
        base_seed_name,
    ).fetch_one(&mut tx).await?.id.unwrap();
    Ok((tx, seed_id))
}

pub async fn upsert_players(
    mut tx: Tx,
    players: &Vec<String>,
) -> Result<(Tx, Vec<i32>)> {
    let mut player_ids = Vec::new();
    for player_name in players {
        player_ids.push(sqlx::query!(
            "with new_players as (
                INSERT INTO players (name) VALUES ($1)
                ON CONFLICT (name) DO NOTHING
                RETURNING id
            ) select coalesce(
                (select id from new_players)
              , (select id from players where name = $1)
            ) id",
            player_name,
        ).fetch_one(&mut tx).await?.id.unwrap());
    }
    Ok((tx, player_ids))
}

pub async fn insert_game(
    mut tx: Tx,
    game: &Game,
    seed_id: i16,
) -> Result<(Tx, i32)> {
    let game_id = sqlx::query!(
        "INSERT INTO games (
            site_game_id
          , seed_id
          , score
          , turns
          , datetime_started
          , datetime_ended
        ) VALUES (
            $1
          , $2
          , $3
          , $4
          , $5
          , $6
        ) returning id",
        game.game_id,
        seed_id,
        game.score,
        game.turns,
        game.datetime_started,
        game.datetime_ended,
    ).fetch_one(&mut tx).await?.id;
    Ok((tx, game_id))
}

pub async fn insert_game_players(
    mut tx: Tx,
    game_id: i32,
    player_ids: &Vec<i32>,
) -> Result<Tx> {
    for player_id in player_ids {
        sqlx::query!(
            "INSERT INTO game_players (
                game_id
              , player_id
            ) VALUES (
                $1
              , $2
            )",
            game_id,
            *player_id,
        ).execute(&mut tx).await?;
    }
    Ok(tx)
}

pub async fn add_seed_games(
    mut tx: Tx,
    seed_games: &SeedGames,
) -> Result<Tx> {
    // This pattern is not the most ergonomic; revisit it if this RFC lands:
    // https://github.com/rust-lang/rfcs/pull/2909
    let tx_and_seed_id = upsert_seed(
        tx,
        seed_games.num_players,
        seed_games.variant_id,
        &seed_games.base_seed_name,
    ).await?;
    tx = tx_and_seed_id.0;
    let seed_id = tx_and_seed_id.1;

    for game in &seed_games.games {
        let tx_and_player_ids = upsert_players(tx, &game.players).await?;
        tx = tx_and_player_ids.0;
        let player_ids = tx_and_player_ids.1;
        let tx_and_game_id = insert_game(
            tx,
            &game,
            seed_id,
        ).await?;
        tx = tx_and_game_id.0;
        let game_id = tx_and_game_id.1;

        tx = insert_game_players(
            tx,
            game_id,
            &player_ids
        ).await?;
    }
    Ok(tx)
}

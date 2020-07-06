use anyhow::Result;
use serde::{Serialize, Deserialize};
use crate::model::{UtcDateTime, Date, Tx};

#[derive(thiserror::Error, Debug)]
pub enum CompetitionGamesError {
    #[error("Consistency error in competition results: {0}")]
    Consistency(String),
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
#[derive(Serialize, Deserialize)]
pub struct SeedGames {
    pub base_seed_name: String,
    pub games: Vec<Game>,
}
#[derive(Serialize, Deserialize)]
pub struct CompetitionGames {
    pub num_players: i16,
    // need the variant ID at the point of retrieving games anyway, since it constitutes part
    // of the full seed name
    pub variant_id: i32,
    pub end_date: Date,
    pub seeds_games: Vec<SeedGames>
}

impl CompetitionGames {
    pub fn validate(&self) -> Result<(), CompetitionGamesError> {
        let num_players = self.num_players;
        for seed_games in &self.seeds_games {
            for game_results in &seed_games.games {
                if game_results.players.len() != num_players as usize
                {
                    return Err(CompetitionGamesError::Consistency(format!(
                        "num_players = {}; players = {:?}", num_players, &game_results.players
                    )));
                }
            }
        }
        Ok(())
    }
}


pub async fn add_competitions_games(
    pool: &super::super::DbAdminPool,
    competitions_games: &Vec<CompetitionGames>,
) -> Result<()> {
    // if a single competition causes an error, don't commit any
    let mut tx = pool.0.begin().await?;
    for games in competitions_games {
        // I'd prefer to use references throughout, but I don't know a better pattern that would
        // allow me to pass the same mutable borrow to multiple functions.
        tx = add_competition_games(tx, games).await?;
    }
    sqlx::query("select update_computed_competition_standings()").execute(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn select_seed_id(
    mut tx: Tx,
    base_seed_name: &str,
    variant_id: i32,
    num_players: i16,
) -> Result<(Tx, i16)> {
    let seed_id = sqlx::query!(
        "SELECT competition_seeds.id
        FROM competition_seeds
        JOIN variants on variant_id = variants.id
        WHERE
            base_name = $1
            and variants.site_variant_id = $2
            and num_players = $3
        ",
        base_seed_name,
        variant_id,
        num_players,
    ).fetch_one(&mut tx).await?.id;
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

pub async fn add_competition_games(
    mut tx: Tx,
    competition_games: &CompetitionGames,
) -> Result<Tx> {
    for seed_games in &competition_games.seeds_games {
        // This pattern is not the most ergonomic; revisit it if this RFC lands:
        // https://github.com/rust-lang/rfcs/pull/2909
        let tx_and_seed_id = select_seed_id(
            tx,
            &seed_games.base_seed_name,
            competition_games.variant_id,
            competition_games.num_players,
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
    }
    Ok(tx)
}

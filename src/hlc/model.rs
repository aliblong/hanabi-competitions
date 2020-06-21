use serde::{Serialize, Deserialize};
use actix_web::{HttpResponse, HttpRequest, Responder, Error};
use futures::future::{ready, Ready};
use sqlx::{PgPool, FromRow, Row};
use sqlx::postgres::PgRow;
use anyhow::Result;
use sqlx::postgres::*;
//use super::routes::CompetitionResultsQueryParams;

// // this struct will use to receive user input
// #[derive(Serialize, Deserialize)]
// pub struct TodoRequest {
//     pub description: String,
//     pub done: bool
// }
// 
// // this struct will be used to represent database record
// #[derive(Serialize, FromRow)]
// pub struct Todo {
//     pub id: i32,
//     pub description: String,
//     pub done: bool,
// }
// 
// // implementation of Actix Responder for Todo struct so we can return Todo from action handler
// impl Responder for Todo {
//     type Error = Error;
//     type Future = Ready<Result<HttpResponse, Error>>;
// 
//     fn respond_to(self, _req: &HttpRequest) -> Self::Future {
//         let body = serde_json::to_string(&self).unwrap();
//         // create response and set content type
//         ready(Ok(
//             HttpResponse::Ok()
//                 .content_type("application/json")
//                 .body(body)
//         ))
//     }
// }

pub async fn find_competition_result_line_items(
    pool: &PgPool,
    query_where_clause: &str,
) -> Result<Vec<CompetitionResult>> {
    let result = sqlx::query_as::<sqlx::Postgres, CompetitionResult>(&format!(r#"
with base_cte as (
    select
        competitions.id competition_id
      , seeds.id seed_id
      , seeds.name seed_name
      , games.id game_id
      , games.score
      , games.turns
      , cast(
            rank() over(partition by seeds.id order by games.score desc, games.turns)
        as smallint) seed_rank
      , cast(count(*) over(partition by seeds.id) as smallint) num_seed_participants
    from competitions
    join competition_seeds on competition_seeds.competition_id = competitions.id
    join seeds on competition_seeds.seed_id = seeds.id
    join games on seeds.id = games.seed_id
),
mp_computed as (
    select
        competition_id
      , seed_id
      , seed_name
      , (
            2 * num_seed_participants
            - (cast(count(*) over(partition by seed_name, seed_rank) as smallint) - 1)
            - 2 * seed_rank
        ) as seed_matchpoints
      , num_seed_participants
      , game_id
      , score
      , turns
    from base_cte
),
mp_agg as (
    select
        competition_id
      , sum(seed_matchpoints) over(partition by competition_id, players.id) as sum_MP
      , 2 * (
            sum(num_seed_participants) over(partition by competition_id)
            - count(seed_id) over(partition by competition_id)
        ) as max_MP
      , players.name player_name
      , seed_id
      , seed_name
      , seed_matchpoints
      , game_id
      , score
      , turns
    from mp_computed
    join game_players using(game_id)
    join players on game_players.player_id = players.id
)
select
    competition_names.name competition_name
  , rank() over(partition by competition_id order by sum_MP) final_rank
  , cast(sum_MP as real)/ max_MP as fractional_MP
  , sum_MP
  , player_name
  , seed_name
  , seed_matchpoints
  , game_id
  , score
  , turns
  , characters.name character_name
from mp_agg
join competition_names on competition_id = competition_names.id
left join seed_characters on mp_agg.seed_id = seed_characters.character_id
left join characters on seed_characters.character_id = characters.id
-- intentional sql injection, so make sure account doesn't have any more
-- privileges than select
where {}
order by
    competition_name desc
  , sum_MP desc
  , seed_name
  , game_id
  , player_name
        "#, query_where_clause))
        .fetch_all(&*pool)
        .await?;
    Ok(result)
}

#[derive(sqlx::FromRow)]
struct CompetitionResult {
    competition_name: String,
    final_rank: i64,
    fractional_mp: f64,
    sum_mp: i64,
    player_name: String,
    seed_name: String,
    seed_matchpoints: i32,
    game_id: i64,
    score: i16,
    turns: i16,
    character_name: Option<String>,
}

/*
        let nums_of_players_bind_fragment = vec!["?"; query_params.nums_of_players.len()].as_slice().join(",");
        let records = sqlx::query(
            qs
        )
            .fetch_all(pool)
            .await?;

        for rec in recs {
            todos.push(Todo {
                id: rec.id,
                description: rec.description,
                done: rec.done
            });
        }

        Ok(todos)
    }

    pub async fn find_by_id(id: i32, pool: &PgPool) -> Result<Todo> {
        let rec = sqlx::query!(
                r#"
                    SELECT * FROM todos WHERE id = $1
                "#,
                id
            )
            .fetch_one(&*pool)
            .await?;

        Ok(Todo {
            id: rec.id,
            description: rec.description,
            done: rec.done
        })
    }

    pub async fn create(todo: TodoRequest, pool: &PgPool) -> Result<Todo> {
        let mut tx = pool.begin().await?;
        let todo = sqlx::query("INSERT INTO todos (description, done) VALUES ($1, $2) RETURNING id, description, done")
            .bind(&todo.description)
            .bind(todo.done)
            .map(|row: PgRow| {
                Todo {
                    id: row.get(0),
                    description: row.get(1),
                    done: row.get(2)
                }
            })
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(todo)
    }

    pub async fn update(id: i32, todo: TodoRequest, pool: &PgPool) -> Result<Todo> {
        let mut tx = pool.begin().await.unwrap();
        let todo = sqlx::query("UPDATE todos SET description = $1, done = $2 WHERE id = $3 RETURNING id, description, done")
            .bind(&todo.description)
            .bind(todo.done)
            .bind(id)
            .map(|row: PgRow| {
                Todo {
                    id: row.get(0),
                    description: row.get(1),
                    done: row.get(2)
                }
            })
            .fetch_one(&mut tx)
            .await?;

        tx.commit().await.unwrap();
        Ok(todo)
    }

    pub async fn delete(id: i32, pool: &PgPool) -> Result<u64> {
        let mut tx = pool.begin().await?;
        let deleted = sqlx::query("DELETE FROM todos WHERE id = $1")
            .bind(id)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(deleted)
    }
}
*/

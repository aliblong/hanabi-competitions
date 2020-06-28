use serde::{Serialize, Deserialize};
use actix_web::{HttpResponse, HttpRequest, Responder, Error, web};
use futures::future::{ready, Ready};
use sqlx::{PgPool, FromRow, Row};
use sqlx::postgres::PgRow;
use anyhow::Result;
use sqlx::postgres::*;
use chrono::{Weekday, Duration, Datelike};
//use super::routes::CompetitionResultsQueryParams;

pub type UtcDateTime = chrono::DateTime<chrono::offset::Utc>;
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
//: web::Json<TodoRequest>
#[derive(Serialize, Deserialize)]
pub struct Variant {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct PartiallySpecifiedCompetition {
    pub num_players: i16,
    pub variant: String,
    pub end_time: Option<UtcDateTime>,
    pub deckplay_enabled: Option<bool>,
    pub empty_clues_enabled: Option<bool>,
    pub characters_enabled: Option<bool>,
    pub additional_rules: Option<String>,
    pub seeds: Option<Vec<String>>,
}
#[derive(Serialize, Deserialize)]
pub struct Competition {
    pub num_players: i16,
    pub variant: String,
    pub end_time: UtcDateTime,
    pub deckplay_enabled: bool,
    pub empty_clues_enabled: bool,
    pub characters_enabled: bool,
    pub additional_rules: String,
    pub seeds: Vec<String>,
}

impl PartiallySpecifiedCompetition {
    pub fn fill_missing_values_with_defaults(mut self) -> Competition {
        if self.end_time.is_none() {
            let today = chrono::offset::Utc::today();
            // If I'm setting the competition on Mon-Wed, it's probably the current competition
            let days_to_add = match today.weekday() {
                Weekday::Mon => 14,
                Weekday::Tue => 13,
                Weekday::Wed => 12,
                Weekday::Thu => 18,
                Weekday::Fri => 17,
                Weekday::Sat => 16,
                Weekday::Sun => 15,
            };
            // Default competition end time: Monday @ 13:00 UTC, which is just after I wake up
            self.end_time = Some((today + Duration::days(days_to_add)).and_hms(13, 0, 0));
        }
        if self.deckplay_enabled.is_none() { self.deckplay_enabled = Some(true) }
        if self.empty_clues_enabled.is_none() { self.empty_clues_enabled = Some(true) }
        if self.characters_enabled.is_none() { self.characters_enabled = Some(true) }
        if self.additional_rules.is_none() { self.additional_rules = Some("".to_owned()) }
        if self.seeds.is_none() {
            let base_seed_name = format!(
                "hl-comp-{}", self.end_time.unwrap().date().format("%Y-%m-%d")
            );
            self.seeds = Some(vec![
                format!("{}-1", base_seed_name),
                format!("{}-2", base_seed_name),
                format!("{}-3", base_seed_name),
                format!("{}-4", base_seed_name),
            ]);
        }
        Competition {
            num_players: self.num_players,
            variant: self.variant,
            end_time: self.end_time.unwrap(),
            deckplay_enabled: self.deckplay_enabled.unwrap(),
            empty_clues_enabled: self.empty_clues_enabled.unwrap(),
            characters_enabled: self.characters_enabled.unwrap(),
            additional_rules: self.additional_rules.unwrap(),
            seeds: self.seeds.unwrap(),
        }
    }
}

pub async fn add_variant(
    pool: &super::super::DbAdminPool,
    variant: Variant,
) -> Result<()> {
    let mut tx = pool.0.begin().await?;
    sqlx::query!(
        "INSERT INTO variants (
            id
          , name
        ) VALUES (
            $1
          , $2
        )",
        variant.id,
        variant.name,
    ).execute(&mut tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn add_competition(
    pool: &super::super::DbAdminPool,
    partially_specified_competition: PartiallySpecifiedCompetition,
) -> Result<()> {
    let competition = partially_specified_competition.fill_missing_values_with_defaults();
    let variant_id = sqlx::query!(
        "SELECT id from variants WHERE name = $1",
        competition.variant
    ).fetch_one(&pool.0).await?.id;

    let mut tx = pool.0.begin().await?;
    let competition_id = sqlx::query!(
        "INSERT INTO competitions (
            end_time
          , num_players
          , variant_id
          , deckplay_enabled
          , empty_clues_enabled
          , characters_enabled
          , additional_rules
        ) VALUES (
            $1
          , $2
          , $3
          , $4
          , $5
          , $6
          , $7
        ) RETURNING id",
        competition.end_time,
        competition.num_players,
        variant_id,
        competition.deckplay_enabled,
        competition.empty_clues_enabled,
        competition.characters_enabled,
        competition.additional_rules,
    ).fetch_one(&mut tx).await?.id;

    for seed in competition.seeds {
        sqlx::query!(
            "INSERT INTO competition_seeds (
                competition_id
              , name
            ) VALUES (
                $1
              , $2
            )",
            competition_id,
            seed,
        ).execute(&mut tx).await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn find_competition_result_line_items(
    pool: &super::super::DbViewerPool,
    query_where_clause: Option<String>,
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
              , sum_MP desc
              , seed_name
              , game_id
              , player_name
        "#, clause_to_insert))
        .fetch_all(&pool.0)
        .await?;
    Ok(result)
}

pub async fn competition_results_to_line_item_json(results: Vec<CompetitionResult>) -> Result<String> {
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
    pub seed_name: String,
    pub seed_matchpoints: i32,
    pub game_id: i64,
    pub score: i16,
    pub turns: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime_started: Option<UtcDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datetime_ended: Option<UtcDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_name: Option<String>,
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

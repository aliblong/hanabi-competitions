use crate::hlc::{Todo, TodoRequest};
use actix_web::{delete, get, post, put, web, HttpResponse, Responder};
use sqlx::PgPool;
use serde::Deserialize;
use serde_qs::actix::QsQuery;

//#[get("/todos")]
//async fn find_all(db_pool: web::Data<PgPool>) -> impl Responder {
//    let result = Todo::find_all(db_pool.get_ref()).await;
//    match result {
//        Ok(todos) => HttpResponse::Ok().json(todos),
//        _ => HttpResponse::BadRequest().body("Error trying to read all todos from database")
//    }
//}
//
//#[derive(Deserialize)]
//pub struct CompetitionResultsQueryParams {
//    pub competition_name_patterns: Option<Vec<String>>,
//    pub variant_name_patterns: Option<Vec<String>>,
//    pub nums_of_players: Option<Vec<u8>>,
//    pub empty_clues_enabled: Option<bool>,
//    pub deckplay_enabled: Option<bool>,
//    pub characters_enabled: Option<bool>,
//    pub characters: Option<Vec<String>>,
//}
//
//impl CompetitionResultsQueryParams {
//    pub fn is_empty(&self) -> bool {
//        self.competition_name_patterns.is_none()
//        && self.variant_name_patterns.is_none()
//        && self.nums_of_players.is_none()
//        && self.empty_clues_enabled.is_none()
//        && self.deckplay_enabled.is_none()
//        && self.characters_enabled.is_none()
//        && self.characters.is_none()
//    }
//}

#[get("/competition")]
async fn get_competition_results(
    query_params: QsQuery<String>,
    db_pool: web::Data<PgPool>
) -> impl Responder {
    let result = super::model::find_competition_result_line_items(db_pool.get_ref(), &query_params.into_inner()).await;
    match result {
        Ok(todo) => HttpResponse::Ok().json(todo),
        _ => HttpResponse::BadRequest().body("Todo not found")
    }
}

//#[post("/todo")]
//async fn create(todo: web::Json<TodoRequest>, db_pool: web::Data<PgPool>) -> impl Responder {
//    let result = Todo::create(todo.into_inner(), db_pool.get_ref()).await;
//    match result {
//        Ok(todo) => HttpResponse::Ok().json(todo),
//        _ => HttpResponse::BadRequest().body("Error trying to create new todo")
//    }
//}
//
//#[put("/todo/{id}")]
//async fn update(id: web::Path<i32>, todo: web::Json<TodoRequest>, db_pool: web::Data<PgPool>) -> impl Responder {
//    let result = Todo::update(id.into_inner(), todo.into_inner(),db_pool.get_ref()).await;
//    match result {
//        Ok(todo) => HttpResponse::Ok().json(todo),
//        _ => HttpResponse::BadRequest().body("Todo not found")
//    }
//}
//
//#[delete("/todo/{id}")]
//async fn delete(id: web::Path<i32>, db_pool: web::Data<PgPool>) -> impl Responder {
//    let result = Todo::delete(id.into_inner(), db_pool.get_ref()).await;
//    match result {
//        Ok(rows) => {
//            if rows > 0 {
//                HttpResponse::Ok().body(format!("Successfully deleted {} record(s)", rows))
//            } else {
//                HttpResponse::BadRequest().body("Todo not found")
//            }
//        },
//        _ => HttpResponse::BadRequest().body("Todo not found")
//    }
//}
//
// function that will be called on new Application to configure routes for this module
pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_competition_results);
}

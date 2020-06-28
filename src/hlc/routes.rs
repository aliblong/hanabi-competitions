use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Error, HttpRequest};
use sqlx::PgPool;
use serde::Deserialize;
use serde_qs::actix::QsQuery;
use actix_http::http::header::Header;
use actix_web_httpauth::headers::authorization::{self, Scheme};
use std::collections::HashMap;

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

pub struct CompetitionResultsQueryParams {
    pub competition_name: String,
    pub raw: Option<bool>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResultLineItemsQueryParams {
    pub where_clause: Option<String>,
    pub raw: Option<bool>,
}

#[get("/result_line_items")]
async fn find_competition_results_with_arbitrary_where_clause(
    query_params: QsQuery<ResultLineItemsQueryParams>,
    db_pool: web::Data<super::super::DbViewerPool>
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let where_clause = unwrapped_query_params.where_clause;
    let raw_output_flag = unwrapped_query_params.raw;
    let result = super::model::find_competition_result_line_items(
        &db_pool.get_ref(),
        where_clause,
    ).await;
    match result {
        Ok(results) => {
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                Ok(HttpResponse::Ok().json(
                        super::model::competition_results_to_line_item_json(results).await.unwrap()))
            } else {
                Ok(HttpResponse::Ok()
                    //.header("LOCATION", "/static/lobby_browser.html")
                    .content_type("text/html; charset=utf-8")
                    .body(include_str!("../../static/result_line_items.html")))
            }
        }
        Err(err) => {
            println!("{:?}", err);
            Ok(HttpResponse::BadRequest().body("Malformed 'where' clause"))
        }
    }
}

#[post("/competition")]
async fn add_competition(
    req: HttpRequest,
    wrapped_db_pool: web::Data<super::super::DbAdminPool>,
    wrapped_api_credentials: web::Data<super::super::ApiCredentials>,
    wrapped_json_payload: web::Json<super::model::PartiallySpecifiedCompetition>,
) -> Result<HttpResponse, Error> {
    let auth = authorization::Authorization::<authorization::Basic>::parse(&req)?.into_scheme();
    let supplied_pw = auth.password();
    if supplied_pw.is_none() {
        return Ok(HttpResponse::Unauthorized().body("Missing password in credentials"));
    }
    let api_credentials = &wrapped_api_credentials.into_inner().0;
    let stored_pw = api_credentials.get(auth.user_id() as &str);
    if stored_pw.is_none() || stored_pw.unwrap() != supplied_pw.unwrap() {
        return Ok(HttpResponse::Unauthorized().body("Bad credentials"));
    }
    let result = super::model::add_competition(
        &wrapped_db_pool.into_inner(),
        wrapped_json_payload.into_inner()
    ).await;
    match result {
        Ok(_) => Ok(HttpResponse::Ok().body("Values were successfully inserted.")),
        Err(_) => Ok(HttpResponse::Ok().body("Malformed input; values were not inserted.")),
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
    cfg.service(find_competition_results_with_arbitrary_where_clause);
}

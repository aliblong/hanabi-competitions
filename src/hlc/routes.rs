use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Error};
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
    db_pool: web::Data<super::super::DbViewerPool>,
    api_passwords: web::Data<Vec<String>>,
    wrapped_json_payload: web::Json<PartiallySpecifiedCompetition>,
) -> Result<HttpResponse, Error> {
    let json_payload = wrapped_json_payload.into_inner();
    if api_passwords.iter().find(json_payload.api_password).is_none() {
        Ok(
    }
    super::model::add_competition()
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

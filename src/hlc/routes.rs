use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Error, HttpRequest};
use sqlx::PgPool;
use serde::Deserialize;
use serde_qs::actix::QsQuery;
use actix_http::http::header::Header;
use actix_web_httpauth::headers::authorization::{self, Scheme};
use std::collections::HashMap;
use thiserror;

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

#[derive(thiserror::Error, Debug)]
pub enum InsertError {
    #[error("values could not be inserted")]
    Values,
    #[error(transparent)]
    Credentials(CredentialsError),
}
impl From<CredentialsError> for InsertError {
    fn from(err: CredentialsError) -> InsertError {
        InsertError::Credentials(err)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CredentialsError {
    #[error("credentials couldn't be parsed")]
    Parse,
    #[error("password was not supplied in credentials")]
    MissingPassword,
    #[error("credentials did not match any known admin")]
    BadCredentials,
}

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

async fn authenticate(
    req: &HttpRequest,
    admin_credentials: &super::super::AdminCredentials,
) -> Result<bool, CredentialsError> {
    match authorization::Authorization::<authorization::Basic>::parse(req) {
        Err(_) => Err(CredentialsError::Parse.into()),
        Ok(credentials_str) => {
            let credentials = credentials_str.into_scheme();
            let supplied_pw = credentials.password();
            if supplied_pw.is_none() {
                return Err(CredentialsError::MissingPassword.into());
            }
            let stored_pw = admin_credentials.0.get(credentials.user_id() as &str);
            let are_credentials_valid = !(stored_pw.is_none() || stored_pw.unwrap() != supplied_pw.unwrap());
            Ok(are_credentials_valid)
        }
    }
}

#[post("/variant")]
async fn add_variants(
    req: HttpRequest,
    wrapped_db_pool: web::Data<super::super::DbAdminPool>,
    wrapped_admin_credentials: web::Data<super::super::AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<super::model::Variant>>,
) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().body("Variants were successfully inserted."))
}


#[post("/competition")]
async fn add_competitions(
    req: HttpRequest,
    wrapped_db_pool: web::Data<super::super::DbAdminPool>,
    wrapped_admin_credentials: web::Data<super::super::AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<super::model::PartiallySpecifiedCompetition>>,
) -> Result<HttpResponse, Error> {
    match authenticate(&req, &wrapped_admin_credentials.into_inner()).await {
        Ok(true) => (),
        Ok(false) => {
            return Ok(HttpResponse::Unauthorized().body("Bad credentials"));
        }
        Err(err) => {
            return Ok(HttpResponse::Unauthorized().body(format!("{}", err)));
        }
    }
    let db_pool = wrapped_db_pool.into_inner();
    for competition in wrapped_json_payload.into_inner() {
        match super::model::add_competition(&db_pool, competition).await {
            Ok(_) => (),
            Err(err) => return Ok(HttpResponse::Unauthorized().body(format!("{}", err))),
        }
    }
    Ok(HttpResponse::Ok().body("Competitions and seeds were successfully inserted."))
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
    cfg.service(add_competitions);
}

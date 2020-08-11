use actix_web::{post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::game::{add_competitions_games, CompetitionGames},
    routes::{authenticate, AdminCredentials},
    DbAdminPool,
};

#[post("/games")]
async fn post_games(
    req: HttpRequest,
    wrapped_db_pool: web::Data<DbAdminPool>,
    wrapped_admin_credentials: web::Data<AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<CompetitionGames>>,
) -> Result<HttpResponse, Error> {
    match authenticate(&req, &wrapped_admin_credentials.into_inner()).await {
        Err(resp) => return Ok(resp.build_credentials_error_response()),
        Ok(_) => (),
    }
    let competitions_results = wrapped_json_payload.into_inner();
    for competition_results in &competitions_results {
        match competition_results.validate() {
            Ok(_) => (),
            Err(_) => return Ok(HttpResponse::BadRequest().body("Competition results are malformed.")),
        }
    }
    match add_competitions_games(
        &wrapped_db_pool.into_inner(),
        &competitions_results
    ).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Games were successfully inserted.")),
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
    }
}

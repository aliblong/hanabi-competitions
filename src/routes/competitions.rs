pub mod results;

use actix_web::{post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::competition::{add_competitions, PartiallySpecifiedCompetition},
    routes::{authenticate, AdminCredentials},
    DbAdminPool,
};

#[post("/competitions")]
async fn post_competitions(
    req: HttpRequest,
    wrapped_db_pool: web::Data<DbAdminPool>,
    wrapped_admin_credentials: web::Data<AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<PartiallySpecifiedCompetition>>,
) -> Result<HttpResponse, Error> {
    match authenticate(&req, &wrapped_admin_credentials.into_inner()).await {
        Err(resp) => return Ok(resp.build_credentials_error_response()),
        Ok(_) => (),
    }
    match add_competitions(
        &wrapped_db_pool.into_inner(),
        wrapped_json_payload.into_inner()
    ).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Competitions and seeds were successfully inserted.")),
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
    }
}

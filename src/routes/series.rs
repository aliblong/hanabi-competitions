use actix_web::{post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::series::{add_series, Series},
    routes::{authenticate, AdminCredentials},
    DbAdminPool,
};

#[post("/series")]
async fn post_series(
    req: HttpRequest,
    wrapped_db_pool: web::Data<DbAdminPool>,
    wrapped_admin_credentials: web::Data<AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<Series>>,
) -> Result<HttpResponse, Error> {
    match authenticate(&req, &wrapped_admin_credentials.into_inner()).await {
        Err(resp) => return Ok(resp.build_credentials_error_response()),
        Ok(_) => (),
    }
    match add_series(
        &wrapped_db_pool.into_inner(),
        wrapped_json_payload.into_inner()
    ).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Series were successfully inserted.")),
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
    }
}

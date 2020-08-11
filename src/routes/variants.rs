use actix_web::{post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::variant::{add_variants, Variant},
    routes::{authenticate, AdminCredentials},
    DbAdminPool,
};

#[post("/variant")]
async fn post_variants(
    req: HttpRequest,
    wrapped_db_pool: web::Data<DbAdminPool>,
    wrapped_admin_credentials: web::Data<AdminCredentials>,
    wrapped_json_payload: web::Json<Vec<Variant>>,
) -> Result<HttpResponse, Error> {
    match authenticate(&req, &wrapped_admin_credentials.into_inner()).await {
        Err(resp) => return Ok(resp.build_credentials_error_response()),
        Ok(_) => (),
    }
    match add_variants(
        &wrapped_db_pool.into_inner(),
        &wrapped_json_payload.into_inner(),
    ).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Variants were successfully inserted.")),
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
    }
}


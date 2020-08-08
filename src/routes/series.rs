use actix_web::{get, post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::series::{add_series, Series},
    routes::{authenticate, AdminCredentials},
    DbViewerPool,
    DbAdminPool,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SeriesQueryParams {
    pub raw: Option<bool>,
    pub max_num_comps: Option<u8>,
}

#[get("/series/{name}")]
async fn get_series(
    query_params: serde_qs::actix::QsQuery<SeriesQueryParams>,
    wrapped_series_name: web::Path<String>,
    db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let series_name = wrapped_series_name.into_inner();
    let raw_output_flag = unwrapped_query_params.raw;
    let max_num_comps = match unwrapped_query_params.max_num_comps {
        Some(max_num_comps) => max_num_comps,
        None => 16,
    };
    match crate::model::series::get_series_view(&db_pool.get_ref(), &series_name, max_num_comps as i64).await {
        Ok(results) => {
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                Ok(HttpResponse::Ok().json(serde_json::to_string(&results).unwrap()))
            } else {
                Ok(HttpResponse::Ok()
                    //.header("LOCATION", "/static/lobby_browser.html")
                    .content_type("text/html; charset=utf-8")
                    .body(hb.render("series", &results).unwrap()))
            }
        }
        Err(err) => {
            println!("{:?}", err);
            Ok(HttpResponse::BadRequest().body("Malformed request"))
        }
    }
}

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

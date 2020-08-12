use actix_web::{get, post, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::competition::{
        add_competitions,
        PartiallySpecifiedCompetition,
        get_competition_and_nested_results
    },
    routes::{authenticate, AdminCredentials},
    DbViewerPool,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompetitionNestedQueryParams {
    pub raw: Option<bool>,
}

#[get("/competitions/{name}")]
async fn get_competition(
    query_params: serde_qs::actix::QsQuery<CompetitionNestedQueryParams>,
    wrapped_competition_name: web::Path<String>,
    db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let competition_name = wrapped_competition_name.into_inner();
    let raw_output_flag = unwrapped_query_params.raw;
    match get_competition_and_nested_results(
        &db_pool.get_ref(),
        &competition_name,
    ).await {
        Ok(results) => {
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                Ok(HttpResponse::Ok().json(serde_json::to_string(&results).unwrap()))
            } else {
                Ok(HttpResponse::Ok()
                    //.header("LOCATION", "/static/lobby_browser.html")
                    .content_type("text/html; charset=utf-8")
                    .body(hb.render("competition", &results).unwrap()))
            }
        }
        Err(err) => {
            Ok(HttpResponse::BadRequest().body(format!("{}", err)))
        }
    }
}

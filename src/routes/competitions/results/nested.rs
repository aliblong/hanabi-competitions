use actix_web::{get, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::competition::results::nested::{
        get_single_competition_results,
        flat_results_to_json,
        nested_results_from_single_competition_results,
    }, 
    //CompetitionResult, },
    DbViewerPool,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompetitionNestedQueryParams {
    pub raw: Option<bool>,
}

#[get("/competitions/results/nested/{name}")]
async fn get_competition_results_nested(
    query_params: serde_qs::actix::QsQuery<CompetitionNestedQueryParams>,
    wrapped_competition_name: web::Path<String>,
    db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let competition_name = wrapped_competition_name.into_inner();
    let raw_output_flag = unwrapped_query_params.raw;
    match get_single_competition_results(
        &db_pool.get_ref(),
        &competition_name,
    ).await {
        Ok((results, team_size)) => {
            let nested_results = nested_results_from_single_competition_results(results, team_size);
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                Ok(HttpResponse::Ok().json(serde_json::to_string(&nested_results).unwrap()))
            } else {
                Ok(HttpResponse::Ok()
                    //.header("LOCATION", "/static/lobby_browser.html")
                    .content_type("text/html; charset=utf-8")
                    .body(hb.render("results_nested", &nested_results).unwrap()))
            }
        }
        Err(err) => {
            println!("{:?}", err);
            Ok(HttpResponse::BadRequest().body("Malformed request"))
        }
    }
}

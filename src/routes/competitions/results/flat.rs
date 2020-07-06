use actix_web::{get, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::competition::results::flat::{get_arbitrary_competitions_results, flat_results_to_json}, 
    //CompetitionResult, },
    DbViewerPool,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompetitionFlatQueryParams {
    pub where_clause: Option<String>,
    pub raw: Option<bool>,
}

#[get("/competitions/results/flat")]
async fn get_competitions_flat(
    query_params: serde_qs::actix::QsQuery<CompetitionFlatQueryParams>,
    db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let where_clause = unwrapped_query_params.where_clause;
    let raw_output_flag = unwrapped_query_params.raw;
    let result = get_arbitrary_competitions_results(
        &db_pool.get_ref(),
        &where_clause,
    ).await;
    match result {
        Ok(results) => {
            //println!("{}", json_results);
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                let json_results = flat_results_to_json(results).await.unwrap();
                Ok(HttpResponse::Ok().json(json_results))
            } else {
                Ok(HttpResponse::Ok()
                    //.header("LOCATION", "/static/lobby_browser.html")
                    .content_type("text/html; charset=utf-8")
                    .body(hb.render("results_flat", &results).unwrap())
                )
            }
        }
        Err(err) => {
            println!("{:?}", err);
            Ok(HttpResponse::BadRequest().body("Malformed 'where' clause"))
        }
    }
}

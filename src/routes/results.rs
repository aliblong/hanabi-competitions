use actix_web::{get, web, HttpResponse, Error};
use crate::{
    model::result::get_combined_results, 
    //CompetitionResult, },
    DbViewerPool,
};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ResultsQueryParams {
    pub where_clause: Option<String>,
    pub raw: Option<bool>,
}

#[get("/results")]
async fn get_results(
    query_params: serde_qs::actix::QsQuery<ResultsQueryParams>,
    db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let unwrapped_query_params = query_params.into_inner();
    let where_clause = unwrapped_query_params.where_clause;
    let raw_output_flag = unwrapped_query_params.raw;
    match get_combined_results(&db_pool.get_ref(), &where_clause).await {
        Ok(results) => {
            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
                Ok(HttpResponse::Ok().json(results))
            } else {
                Ok(HttpResponse::Ok()
                    .content_type("text/html; charset=utf-8")
                    .body(hb.render("combined_results", &results).unwrap())
                )
            }
        }
        Err(err) => {
            info!("{:?}", err);
            Ok(HttpResponse::BadRequest().body("Malformed 'where' clause"))
        }
    }
}

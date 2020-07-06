
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CompetitionNestedQueryParams {
    pub competition_name: String,
    pub raw: Option<bool>,
}

//#[get("/competition/results/nested")]
//async fn get_competition_results_nested(
//    query_params: QsQuery<CompetitionNestedQueryParams>,
//    db_pool: web::Data<super::super::DbViewerPool>
//) -> Result<HttpResponse, Error> {
//    let unwrapped_query_params = query_params.into_inner();
//    let competition_name = unwrapped_query_params.competition_name;
//    let raw_output_flag = unwrapped_query_params.raw;
//    let result = super::model::find_competition_result_line_items(
//        &db_pool.get_ref(),
//        where_clause,
//    ).await;
//    match result {
//        Ok(results) => {
//            if raw_output_flag.is_some() && raw_output_flag.unwrap() {
//                Ok(HttpResponse::Ok().json(
//                        super::model::competition_results_to_line_item_json(results).await.unwrap()))
//            } else {
//                Ok(HttpResponse::Ok()
//                    //.header("LOCATION", "/static/lobby_browser.html")
//                    .content_type("text/html; charset=utf-8")
//                    .body(include_str!("../../static/result_line_items.html")))
//            }
//        }
//        Err(err) => {
//            println!("{:?}", err);
//            Ok(HttpResponse::BadRequest().body("Malformed 'where' clause"))
//        }
//    }
//}

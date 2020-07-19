use actix_web::{get, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::competition::{
        get_competition_names,
        get_active_competitions,
        group_competitions_by_series,
        CompetitionsGroupedBySeries,
    },
    DbViewerPool,
};

#[derive(serde::Serialize)]
struct IndexContents {
    pub competition_names: Vec<String>,
    pub active_series_competitions: CompetitionsGroupedBySeries,
}

#[get("/")]
async fn get_index(
    wrapped_db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let db_pool = &wrapped_db_pool.into_inner();
    match get_competition_names(db_pool).await {
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
        Ok(competition_names) => {
            match get_active_competitions(db_pool).await {
                Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
                Ok(active_competitions) => {
                    let index_contents = IndexContents {
                        competition_names,
                        active_series_competitions:
                            group_competitions_by_series(active_competitions),
                    };
                    Ok(HttpResponse::Ok()
                        .content_type("text/html; charset=utf-8")
                        .body(hb.render("index", &index_contents).unwrap())
                    )
                }
            }
        },
    }
}

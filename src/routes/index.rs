use actix_web::{get, web, HttpResponse, Error, HttpRequest};
use crate::{
    model::{
        series::get_series_names,
        competition::{
            get_competition_names,
            get_active_competitions,
            CompetitionWithDerivedQuantities
        },
    },
    DbViewerPool,
};

#[derive(serde::Serialize, Debug)]
struct IndexContents {
    pub series_names: Vec<String>,
    pub competition_names: Vec<String>,
    pub active_competitions: Vec<CompetitionWithDerivedQuantities>
}

#[get("/")]
async fn get_index(
    wrapped_db_pool: web::Data<DbViewerPool>,
    hb: web::Data<handlebars::Handlebars<'_>>,
) -> Result<HttpResponse, Error> {
    let db_pool = &wrapped_db_pool.into_inner();
    match get_series_names(db_pool).await {
        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
        Ok(series_names) => {
            match get_competition_names(db_pool).await {
                Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
                Ok(competition_names) => {
                    match get_active_competitions(db_pool).await {
                        Err(err) => Ok(HttpResponse::BadRequest().body(format!("{}", err))),
                        Ok(active_competitions) => {
                            let index_contents = IndexContents {
                                series_names,
                                competition_names,
                                active_competitions,
                            };
                            Ok(HttpResponse::Ok()
                                .content_type("text/html; charset=utf-8")
                                .body(hb.render("index", &index_contents).unwrap())
                            )
                        }
                    }
                },
            }
        },
    }
}

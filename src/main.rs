#![recursion_limit="60"]
#[macro_use]
extern crate log;

// import todo module (routes and model)
mod routes;
mod model;

use dotenv::dotenv;
use itertools;
use listenfd::ListenFd;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, FromRequest};
use sqlx::PgPool;
use std::env;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Clone)]
pub struct DbViewerPool(pub PgPool);
#[derive(Clone)]
pub struct DbAdminPool(pub PgPool);

fn get_expected_env_var(name: &str) -> String {
    env::var(name).expect(&*format!("{} must be set (check `.env`)", name))
}

// default / handler
async fn index() -> impl Responder {
    HttpResponse::Ok().body(r#"
        Welcome to Actix-web with SQLx Todos example.
        Available routes:
        GET /todos -> list of all todos
        POST /todo -> create new todo, example: { "description": "learn actix and sqlx", "done": false }
        GET /todo/{id} -> show one todo with requested id
        PUT /todo/{id} -> update todo with requested id, example: { "description": "learn actix and sqlx", "done": true }
        DELETE /todo/{id} -> delete todo with requested id
    "#
    )
}

#[actix_rt::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    // this will enable us to keep application running during recompile: systemfd --no-pid -s http::5000 -- cargo watch -x run
    let mut listenfd = ListenFd::from_env();

    let database_viewer_url = get_expected_env_var("DATABASE_VIEWER_URL");
    let database_admin_url = get_expected_env_var("DATABASE_ADMIN_URL");
    let db_viewer_pool = DbViewerPool(PgPool::new(&database_viewer_url).await?);
    let db_admin_pool = DbAdminPool(PgPool::new(&database_admin_url).await?);
    let admin_credentials_file_path = get_expected_env_var("ACCEPTED_API_CREDENTIALS");
    let admin_credentials = routes::AdminCredentials::read_credentials_from_file(
        &admin_credentials_file_path)
        .expect(&format!("No file found at path: {}", admin_credentials_file_path));
    let mut handlebars = handlebars::Handlebars::new();
    handlebars
        .register_templates_directory(".html", "./static/templates")
        .unwrap();
    let handlebars_ref = web::Data::new(handlebars);

    let mut server = HttpServer::new(move || {
        App::new()
            .data(db_viewer_pool.clone())
            .data(db_admin_pool.clone())
            .data(admin_credentials.clone())
            .app_data(
                // change json extractor configuration
                web::Json::<Vec<model::variant::Variant>>::configure(|cfg| {
                    cfg.limit(100000)
            }))
            .app_data(
                // change json extractor configuration
                web::Json::<Vec<model::game::CompetitionGames>>::configure(|cfg| {
                    cfg.limit(100000)
            }))
            .app_data(handlebars_ref.clone())
            .route("/", web::get().to(index))
            .configure(routes::init) // init todo routes
            .service(actix_files::Files::new("/static", "static").show_files_listing())
    });

    server = match listenfd.take_tcp_listener(0)? {
        Some(listener) => server.listen(listener)?,
        None => {
            let host = env::var("HOST").expect("HOST is not set in .env file");
            let port = env::var("PORT").expect("PORT is not set in .env file");
            server.bind(format!("{}:{}", host, port))?
        }
    };

    info!("Starting server");
    server.run().await?;

    Ok(())
}

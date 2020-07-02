#![recursion_limit="60"]
#[macro_use]
extern crate log;

use dotenv::dotenv;
use itertools;
use listenfd::ListenFd;
use std::{env, fs, io::{BufReader, prelude::*}};
use actix_web::{web, App, HttpResponse, HttpServer, Responder, FromRequest};
use sqlx::PgPool;
use anyhow::Result;
use std::collections::HashMap;

// import todo module (routes and model)
mod hlc;

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

fn read_credentials_from_file(file_path: &str) -> Result<AdminCredentials> {
    let file = fs::File::open(file_path)?;
    let buf = BufReader::new(file);
    let credentials: HashMap<String, String> = itertools::process_results(
        buf.lines(),
        |lines| {
            lines.filter_map(|line| {
                match line.chars().next() {
                    Some('#') => None,
                    None => None,
                    _ => {
                        let mut tokens = line.splitn(2, ':');
                        let user_id = tokens.next().unwrap().to_owned();
                        let password = tokens.next().unwrap().to_owned();
                        Some((user_id, password))
                    },
                }
            }).collect()
        }
    )?;
    Ok(AdminCredentials(credentials))
}

#[derive(Clone)]
pub struct DbViewerPool(pub PgPool);
#[derive(Clone)]
pub struct DbAdminPool(pub PgPool);
#[derive(Clone)]
pub struct AdminCredentials(pub HashMap<String, String>);

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
    let admin_credentials = read_credentials_from_file(&admin_credentials_file_path)
        .expect(&format!("No file found at path: {}", admin_credentials_file_path));

    let mut server = HttpServer::new(move || {
        App::new()
            .data(db_viewer_pool.clone())
            .data(db_admin_pool.clone())
            .data(admin_credentials.clone())
            .app_data(
                // change json extractor configuration
                web::Json::<Vec<hlc::Variant>>::configure(|cfg| {
                    cfg.limit(100000)
            }))
            .app_data(
                // change json extractor configuration
                web::Json::<Vec<hlc::CompetitionResults>>::configure(|cfg| {
                    cfg.limit(100000)
            }))
            .route("/", web::get().to(index))
            .configure(hlc::init) // init todo routes
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

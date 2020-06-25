#[macro_use]
extern crate log;

use dotenv::dotenv;
use itertools;
use listenfd::ListenFd;
use std::{env, fs, io::{BufReader, prelude::*}};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use sqlx::PgPool;
use anyhow::Result;

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

fn read_passwords_from_file(file_path: &str) -> Result<Vec<String>> {
    let file = fs::File::open(file_path)?;
    let buf = BufReader::new(file);
    let results = itertools::process_results(
        buf.lines(),
        |lines| {
            lines.filter(|line| {
                match line.chars().next() {
                    Some('#') => false,
                    None => false,
                    _ => true,
                }
            }).collect()
        }
    )?;
    Ok(results)
}

#[derive(Clone)]
pub struct DbViewerPool(pub PgPool);
#[derive(Clone)]
pub struct DbAdminPool(pub PgPool);
#[derive(Clone)]
pub struct ApiPasswords(pub Vec<String>);

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
    let api_password_file_path = get_expected_env_var("ACCEPTED_API_PASSWORDS");
    let api_passwords = ApiPasswords(read_passwords_from_file(&api_password_file_path).expect(
        format!("No file found at path: {}", api_password_file_path).as_ref()
    ));

    let mut server = HttpServer::new(move || {
        App::new()
            .data(db_viewer_pool.clone())
            .data(db_admin_pool.clone())
            .data(api_passwords.clone())
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

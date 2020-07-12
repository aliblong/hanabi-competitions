pub mod index;
pub mod competitions;
pub mod games;
pub mod variants;
pub mod results;

use actix_web::{delete, get, post, put, web, HttpResponse, Responder, Error, HttpRequest};
use sqlx::PgPool;
use serde::Deserialize;
use std::{env, fs, io::{BufReader, prelude::*}};
use actix_http::http::header::Header;
use actix_web_httpauth::headers::authorization::{self, Scheme};
use std::collections::HashMap;
use anyhow::Result;
use thiserror;

#[derive(Clone)]
pub struct AdminCredentials(pub HashMap<String, String>);

#[derive(thiserror::Error, Debug)]
pub enum InsertError {
    #[error("values could not be inserted")]
    Values,
    #[error(transparent)]
    Credentials(CredentialsError),
}
impl From<CredentialsError> for InsertError {
    fn from(err: CredentialsError) -> InsertError {
        InsertError::Credentials(err)
    }
}

impl AdminCredentials {
    pub fn read_credentials_from_file(file_path: &str) -> Result<Self> {
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
        Ok(Self(credentials))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum CredentialsError {
    #[error("credentials couldn't be parsed")]
    Parse,
    #[error("password was not supplied in credentials")]
    MissingPassword,
    #[error("credentials did not match any known admin")]
    BadCredentials,
}

async fn authenticate(
    req: &HttpRequest,
    admin_credentials: &AdminCredentials,
) -> Result<(), CredentialsError> {
    match authorization::Authorization::<authorization::Basic>::parse(req) {
        Err(_) => Err(CredentialsError::Parse.into()),
        Ok(credentials_str) => {
            let credentials = credentials_str.into_scheme();
            let supplied_pw = credentials.password();
            if supplied_pw.is_none() {
                return Err(CredentialsError::MissingPassword.into());
            }
            let stored_pw = admin_credentials.0.get(credentials.user_id() as &str);
            let are_credentials_valid = stored_pw.is_some() && stored_pw.unwrap() == supplied_pw.unwrap();
            match are_credentials_valid {
                false => Err(CredentialsError::BadCredentials),
                true => Ok(())
            }
        }
    }
}

impl CredentialsError {
    fn build_credentials_error_response(&self) -> HttpResponse {
        let mut builder = match self {
            CredentialsError::Parse | CredentialsError::MissingPassword => {
                HttpResponse::BadRequest()
            },
            CredentialsError::BadCredentials => HttpResponse::Unauthorized(),
        };
        builder.body(format!("{}", self))
    }
}

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(index::get_index);
    cfg.service(results::get_results);
    cfg.service(competitions::get_competition);
    cfg.service(competitions::post_competitions);
    cfg.service(variants::post_variants);
    cfg.service(games::post_games);
}

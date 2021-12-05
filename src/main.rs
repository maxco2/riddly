mod gist;
mod store;
mod util;

use crate::gist::{backup_to_gist, pull_from_gist};
use crate::store::{MemoryTiddlersStore, Store};
use crate::util::compare_etag_and_response;
use actix_files::NamedFile;
use actix_rt::time;
use actix_web::error::BlockingError;
use actix_web::{
    delete, dev::ServiceRequest, get, middleware, put, web, App, Error, HttpRequest, HttpResponse,
    HttpServer,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use futures::StreamExt;
use lazy_static::lazy_static;
use log::info;
use mime::{APPLICATION_JSON, TEXT_HTML};
use serde_json::Value;
use std::env;
use std::path::Path;
use std::sync::RwLock;
use std::time::Duration;

lazy_static! {
    static ref GLOBAL_STORE: RwLock<MemoryTiddlersStore> = RwLock::new(MemoryTiddlersStore::new());
    static ref GITHUB_GIST_TOKEN: String = env::var("GITHUB_GIST_TOKEN").unwrap();
    static ref GITHUB_GIST_ID: String = env::var("GITHUB_GIST_ID").unwrap();
    static ref WIKI_USER_NAME: String = env::var("WIKI_USER_NAME").unwrap();
    static ref WIKI_USER_PASSWORD: String = env::var("WIKI_USER_PASSWORD").unwrap();
}

#[get("/favicon.ico")]
async fn favicon(_: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().content_type("image/x-icon").body("base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABmJLR0QAAAAAAAD5Q7t/AAACwElEQVQ4y6WTS2gdZRiGn/+fyzlnJufSmOOJpF6SKpiKGK0ajGChFSJ2oVSpboMILhRpcOsdBEG6U3eC0JWKG6GCVEEJtsXiIkQPxdiYNraxzW0mk5lzZub/PxdCUbsSn/3zbt73hf+JuvjpnLjaJ7xtELWrQfRdl0B7mLKP61i+nfuS/W/Mkp3qYmoxQ7fWUUENnAZRt4ta/2lJwlYTVRpML6eSWza6Syix2M0c79BeFqbfpjk1zuCBe6iNnCccHkVXAsRvogc6Q6iKhwQVxHeQRg03qIL22Dh")
}

async fn ok_validator(
    req: ServiceRequest,
    credentials: BasicAuth,
) -> Result<ServiceRequest, Error> {
    if credentials.user_id() == WIKI_USER_NAME.as_str()
        && credentials.password().unwrap_or(credentials.user_id()) == WIKI_USER_PASSWORD.as_str()
    {
        Ok(req)
    } else {
        Err(BlockingError::Error("fuck you").into())
    }
}

#[get("/status")]
async fn status(_: HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(r#"{"username":"max","space":{"recipe":"a"}}"#)
}

#[delete("/bags/a/tiddlers/{title}")]
async fn delete_tiddler(
    _req: HttpRequest,
    title: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut store = GLOBAL_STORE.write().unwrap();
    (*store).delete_tiddler(&*title);
    Ok(HttpResponse::NoContent().body(""))
}

#[delete("/bags/bag/tiddlers/{title}")]
async fn delete_tiddler_guard(
    _req: HttpRequest,
    title: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut store = GLOBAL_STORE.write().unwrap();
    (*store).delete_tiddler(&*title);
    Ok(HttpResponse::NoContent().body(""))
}

#[get("/recipes/a/tiddlers.json")]
async fn get_tiddlers_json(req: HttpRequest) -> HttpResponse {
    let store = GLOBAL_STORE.read().unwrap();
    let global_revision = (*store).global_revision();
    compare_etag_and_response(req, global_revision, (*store).all_tiddlers())
}

#[get("/recipes/a/tiddlers/{title}")]
async fn get_tiddler(req: HttpRequest, title: web::Path<String>) -> Result<HttpResponse, Error> {
    let store = GLOBAL_STORE.read().unwrap();
    if let Some(v) = (*store).get_tiddler(&*title) {
        Ok(compare_etag_and_response(req, v.1, v.0))
    } else {
        Ok(HttpResponse::NotFound().body(""))
    }
}

#[get("/recipes/wiki.json")]
async fn get_wiki_json(_: HttpRequest) -> Result<NamedFile, Error> {
    let path: &Path = Path::new("./data.json");
    let file = NamedFile::open(path)?;
    Ok(file
        .set_content_type(APPLICATION_JSON)
        .use_last_modified(true))
}

#[get("/recipes/backup")]
async fn backup(_: HttpRequest) -> Result<HttpResponse, Error> {
    actix_rt::spawn(async move {
        let json_string: String;
        {
            let store = GLOBAL_STORE.read().unwrap();
            json_string = (*store).to_json_string();
        }
        backup_to_gist(json_string).await;
    });
    Ok(HttpResponse::Ok().body("backup to gist!"))
}

#[put("/recipes/a/tiddlers/{title}")]
async fn put_tiddler(
    _req: HttpRequest,
    mut payload: web::Payload,
    title: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        body.extend_from_slice(&chunk);
    }
    let mut meta = serde_json::from_slice::<Value>(&body)?;
    let mut text = String::new();
    match meta {
        Value::Object(ref mut map) => {
            map.insert("bag".to_string(), Value::String("bag".to_string()));
            map.remove("revision");
            if let Some(sub_fields) = map.remove("fields") {
                match sub_fields {
                    Value::Object(sub_field) => {
                        for (k, v) in sub_field.into_iter() {
                            map.insert(k, v);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(Value::String(t)) = map.remove("text") {
                text = t;
            }
        }
        _ => {}
    }
    let rev: u32;
    {
        let mut store = GLOBAL_STORE.write().unwrap();
        rev = (*store).put_tiddler(title.to_string(), meta, text);
    }
    Ok(HttpResponse::NoContent()
        .header("ETag", format!("\"a/{}/{}:\"", title, rev))
        .body(""))
}

#[get("/")]
async fn index(_: HttpRequest) -> Result<NamedFile, Error> {
    let path: &Path = Path::new("./index.html");
    let file = NamedFile::open(path)?;
    Ok(file.set_content_type(TEXT_HTML).use_last_modified(true))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    actix_rt::spawn(async move {
        let mut last_reversion: u64;
        let mut cur_reversion: u64;
        {
            info!("try pull from gist");
            let wiki_data = pull_from_gist().await;
            if let Some(wiki_data) = wiki_data {
                let mut store = GLOBAL_STORE.write().unwrap();
                if (*store).global_revision_num() < wiki_data.global_revision_num() {
                    (*store) = wiki_data;
                    info!("pull from gist done!");
                }
                last_reversion = (*store).global_revision_num();
            } else {
                let store = GLOBAL_STORE.read().unwrap();
                last_reversion = (*store).global_revision_num();
            }
        }
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            {
                let store = GLOBAL_STORE.read().unwrap();
                cur_reversion = (*store).global_revision_num();
                if cur_reversion != last_reversion {
                    let json_string = (*store).to_json_string();
                    std::mem::drop(store);
                    backup_to_gist(json_string).await;
                    info!("interval backup to gist!");
                    last_reversion = cur_reversion;
                }
            }
        }
    });
    // Get the port number to listen on.
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a number");
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(HttpAuthentication::basic(ok_validator))
            .service(index)
            .service(status)
            .service(favicon)
            .service(get_tiddlers_json)
            .service(put_tiddler)
            .service(get_tiddler)
            .service(delete_tiddler)
            .service(delete_tiddler_guard)
            .service(get_wiki_json)
            .service(backup)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

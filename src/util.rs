use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag};
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse};
use serde_json::Value;

pub fn compare_etag_and_response(req: HttpRequest, revision: String, data: Value) -> HttpResponse {
    let mut response = HttpResponse::Ok();
    let mut matched = false;
    if let Some(t) = req.headers().get("If-None-Match") {
        matched = t.len() > 2 && &t.as_bytes()[1..t.len() - 1] == revision.as_bytes();
    }
    response
        .set(ETag(EntityTag::strong(revision)))
        .set(CacheControl(vec![
            CacheDirective::MustRevalidate,
            CacheDirective::MaxAge(0u32),
        ]))
        .content_type("application/json");
    if matched {
        response.status(StatusCode::NOT_MODIFIED).body("")
    } else {
        response.body(data)
    }
}

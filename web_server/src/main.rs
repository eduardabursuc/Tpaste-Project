use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};
use std::{fs, io};

fn read_file_content(file_path: &str) -> Result<String, io::Error> {
    let content = fs::read_to_string(file_path)?;
    Ok(content)
}

async fn tpaste(req: HttpRequest) -> HttpResponse {
    let id = req.match_info().get("id").unwrap_or_default();

    let content = match read_file_content(format!("../server/data/{}.txt", id).as_str()) {
        Ok(c) => c.replace('\n', "<br>"),
        Err(_) => String::new(),
    };

    let response_body = format!("<html><body><p>{}</p></body></html>", content);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(response_body)
}

async fn user_pastes(req: HttpRequest) -> HttpResponse {
    let user_id = req.match_info().get("user_id").unwrap_or_default();

    let content = match read_file_content(format!("../server/pastes/{}.txt", user_id).as_str()) {
        Ok(c) => c.replace('\n', "<br>"),
        Err(_) => String::new(),
    };

    let response_body = format!("<html><body><ul>{}</ul></body></html>", content);

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(response_body)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .route("/tpaste/{id}", web::get().to(tpaste))
            .route("/{user_id}", web::get().to(user_pastes))
    })
    .bind("127.0.0.1:3030")?
    .run()
    .await
}

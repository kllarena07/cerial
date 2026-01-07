use axum::{
    Router,
    body::Body,
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
};
use pulldown_cmark::{Parser, html};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "pages/"]
struct Asset;

#[derive(RustEmbed)]
#[folder = "templates/"]
struct TemplateAsset;

fn get_content_type(path: &str) -> &'static str {
    if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".gif") {
        "image/gif"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

async fn home() -> Html<String> {
    let mut links = Vec::new();
    for path in Asset::iter() {
        if path.ends_with("/index.md") {
            let name = path.trim_end_matches("/index.md");
            links.push(format!("<li><a href=\"/{}\">{}</a></li>", name, name));
        }
    }
    let template = TemplateAsset::get("index.html").expect("Template not found");
    let template_str = std::str::from_utf8(template.data.as_ref()).unwrap();
    let links_html = links.join("");
    let html = template_str.replace("{links}", &links_html);
    Html(html)
}

async fn serve_file(Path(full_path): Path<String>) -> Result<Response, StatusCode> {
    let parts: Vec<&str> = full_path.splitn(2, '/').collect();
    let name = parts[0];
    let path = parts.get(1).copied().unwrap_or("");
    let file_path = if path.is_empty() {
        format!("{}/index.md", name)
    } else {
        format!("{}/{}", name, path)
    };

    if let Some(file) = Asset::get(&file_path) {
        if file_path.ends_with(".md") {
            let markdown_input = std::str::from_utf8(file.data.as_ref())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let parser = Parser::new(markdown_input);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            let html_output = html_output.replace("/assets/", &format!("/{}/assets/", name));
            let template = TemplateAsset::get("page.html").expect("Template not found");
            let template_str = std::str::from_utf8(&template.data).unwrap();
            let full_html = template_str.replace("{content}", &html_output);
            Ok(Html(full_html).into_response())
        } else {
            let content_type = get_content_type(&file_path);
            Ok(Response::builder()
                .header("content-type", content_type)
                .body(Body::from(file.data))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
        }
    } else {
        let template = TemplateAsset::get("404.html").expect("Template not found");
        let not_found_html = std::str::from_utf8(&template.data).unwrap().to_string();
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "text/html")
            .body(Body::from(not_found_html))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(home))
        .route("/{*full_path}", get(serve_file));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

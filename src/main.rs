use axum::{Router, extract::Path, response::Html, routing::get};
use pulldown_cmark::{Parser, html};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "pages/"] // Your folder with .md files
struct Asset;

async fn render_markdown(Path(name): Path<String>) -> Html<String> {
    // 1. Get the file from the binary
    let path = format!("{}/index.md", name);
    let file = Asset::get(&path).expect("File not found");
    let markdown_input = std::str::from_utf8(file.data.as_ref()).unwrap();

    // 2. Convert Markdown to HTML
    let parser = Parser::new(markdown_input);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // 3. Wrap in a simple layout with some CSS
    Html(format!(
        "<html><body style='font-family: sans-serif; max-width: 800px; margin: auto;'>{}</body></html>",
        html_output
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/{capture}", get(render_markdown));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

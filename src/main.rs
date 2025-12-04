use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

use crate::cerial::Cerial;

mod cerial;

fn handle_client(mut stream: TcpStream) {
    let cerial_parser = Cerial::parse(stream.try_clone().unwrap());

    let body_preview = if cerial_parser.get_body().len() > 100 {
        format!("{}...", &cerial_parser.get_body()[..100])
    } else {
        cerial_parser.get_body().to_string()
    };

    println!(
        "[{}, {} {}]: [{} headers], [{} bytes body]",
        cerial_parser.get_method(),
        cerial_parser.get_path(),
        cerial_parser.get_version_string(),
        cerial_parser.get_headers().len(),
        cerial_parser.get_body().len()
    );

    // Demonstrate new header parsing features
    if let Some(content_type) = cerial_parser.get_content_type() {
        println!("Content-Type: {}", content_type);
    }

    if let Some(charset) = cerial_parser.get_charset() {
        println!("Charset: {}", charset);
    }

    let cookies = cerial_parser.get_cookies();
    if !cookies.is_empty() {
        println!("Cookies: {:?}", cookies);
    }

    if let Some(custom_header) = cerial_parser.get_header_value("custom-header") {
        println!("Custom-Header: {}", custom_header);
    }

    // Demonstrate form data parsing
    if cerial_parser.is_form_data() {
        println!("Form data detected:");
        let form_data = cerial_parser.get_form_data();
        for (key, value) in &form_data {
            println!("  {}: {}", key, value);
        }
    }

    // Demonstrate JSON parsing
    if cerial_parser.is_json() {
        println!("JSON data detected:");
        if let Some(json) = cerial_parser.get_json() {
            println!("  Parsed JSON: {}", json);
            if let Some(name) = cerial_parser.get_json_field("name") {
                println!("  Name field: {}", name);
            }
        }
    }

    // Demonstrate chunked encoding detection
    if cerial_parser.is_chunked() {
        println!("Chunked transfer encoding detected");
    }

    println!("Body preview: {}", body_preview);

    let response =
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 6\r\n\r\nhello\n";
    if let Err(e) = stream.write(response.as_bytes()) {
        eprintln!("[ERROR]: Failed to write response: {}", e);
    }
    if let Err(e) = stream.flush() {
        eprintln!("[ERROR]: Failed to flush stream: {}", e);
    }
}

fn main() {
    let ip_address = "0.0.0.0:3000";

    println!("Starting server on {ip_address}");
    let listener = match TcpListener::bind(ip_address) {
        Ok(listener) => listener,
        Err(e) => panic!("[ERROR]: {}", e),
    };

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream),
            Err(e) => eprintln!("[ERROR]: stream could not be handled. See error {e}"),
        };
    }
}

# Cerial HTTP Parser
## ✍ About
A lightweight HTTP request parser built in Rust that provides structured access to HTTP request components.

**Includes:**
- **Header Parsing**: Structured header storage with duplicate header support
- **Content-Type Parsing**: Extract MIME type and parameters (charset, boundary)
- **Cookie Parsing**: Parse cookies into name-value pairs
- **Query Parameter Parsing**: Extract and parse URL query strings
- **Path Extraction**: Separate path from query parameters
- **HTTP Version Parsing**: Parse HTTP version into major/minor components
- **Body Type Detection**: Automatic detection of content types
- **Form Data Parsing**: URL-encoded form data support
- **JSON Parsing**: Built-in JSON body parsing with serde_json
- **Chunked Transfer Encoding**: Support for chunked HTTP bodies
- **Size Limits**: Configurable limits for headers and body

## Usage

```rust
use cerial::Cerial;
use std::net::TcpStream;

let request = Cerial::parse(stream);
println!("Method: {}", request.get_method());
println!("Path: {}", request.get_path());

if let Some(json) = request.get_json() {
    println!("JSON: {}", json);
}
```

## Testing

```bash
cargo run  # Start server
./simple_test.sh  # Run tests
```

## 👾 Bugs or vulnerabilities

If you find any bugs or vulnerabilities, please contact me on my Twitter using the link below.

_Made with ❤️ by [krayondev](https://x.com/krayondev)_

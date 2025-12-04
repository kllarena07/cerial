use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
};

#[derive(Debug, Clone)]
pub struct HttpVersion {
    pub major: u8,
    pub minor: u8,
}

impl HttpVersion {
    pub fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    pub fn from_string(version_str: &str) -> Option<Self> {
        if let Some(slash_pos) = version_str.find('/') {
            let version_part = &version_str[slash_pos + 1..];
            if let Some(dot_pos) = version_part.find('.') {
                let major = version_part[..dot_pos].parse().ok()?;
                let minor = version_part[dot_pos + 1..].parse().ok()?;
                Some(HttpVersion::new(major, minor))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn to_string(&self) -> String {
        format!("HTTP/{}.{}", self.major, self.minor)
    }
}

pub struct Cerial {
    method: String,
    path: String,
    query: HashMap<String, String>,
    version: HttpVersion,
    headers: HashMap<String, Vec<String>>,
    body: String,
}

impl Cerial {
    pub fn parse(stream: TcpStream) -> Self {
        Self::parse_with_limits(stream, 8192, 1024 * 1024) // 8KB headers, 1MB body
    }

    pub fn parse_with_limits(
        stream: TcpStream,
        max_header_size: usize,
        max_body_size: usize,
    ) -> Self {
        let mut reader = BufReader::new(stream);

        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let mut parts = request_line.trim().split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path_and_query = parts.next().unwrap_or("").to_string();
        let version_str = parts.next().unwrap_or("HTTP/1.1");
        let version = HttpVersion::from_string(version_str).unwrap_or(HttpVersion::new(1, 1));

        // Parse path and query
        let (path, query) = Self::parse_path_and_query(&path_and_query);

        // Parse headers into HashMap with size limit
        let mut headers = HashMap::new();
        let mut body = String::new();
        let mut headers_complete = false;
        let mut header_size = 0;

        while !headers_complete {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }

            header_size += line.len();
            if header_size > max_header_size {
                eprintln!(
                    "Warning: Header size exceeds limit of {} bytes",
                    max_header_size
                );
                break;
            }

            if line.trim().is_empty() {
                headers_complete = true;

                // Read body based on transfer encoding
                if Self::is_chunked_headers(&headers) {
                    body = Self::parse_chunked_body(&mut reader, max_body_size);
                } else if let Some(content_length) = Self::extract_content_length_from_map(&headers)
                {
                    if content_length > max_body_size {
                        eprintln!(
                            "Warning: Body size {} exceeds limit of {} bytes",
                            content_length, max_body_size
                        );
                        // Read only up to the limit
                        let limited_size = max_body_size.min(content_length);
                        let mut body_bytes = vec![0u8; limited_size];
                        reader.read_exact(&mut body_bytes).unwrap();
                        body = String::from_utf8_lossy(&body_bytes).to_string();

                        // Discard the rest of the body
                        let mut discard_bytes = vec![0u8; content_length - limited_size];
                        let _ = reader.read_exact(&mut discard_bytes);
                    } else {
                        let mut body_bytes = vec![0u8; content_length];
                        reader.read_exact(&mut body_bytes).unwrap();
                        body = String::from_utf8_lossy(&body_bytes).to_string();
                    }
                }
            } else {
                // Parse header line
                if let Some(colon_pos) = line.find(':') {
                    let name = line[..colon_pos].trim().to_lowercase();
                    let value = line[colon_pos + 1..].trim().to_string();

                    headers.entry(name).or_insert_with(Vec::new).push(value);
                }
            }
        }

        Cerial {
            method,
            path,
            query,
            version,
            headers,
            body,
        }
    }
    pub fn get_method(&self) -> &str {
        &self.method
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_query(&self) -> &HashMap<String, String> {
        &self.query
    }

    pub fn get_query_param(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    pub fn get_version(&self) -> &HttpVersion {
        &self.version
    }

    pub fn get_version_string(&self) -> String {
        self.version.to_string()
    }

    pub fn get_headers(&self) -> &HashMap<String, Vec<String>> {
        &self.headers
    }

    pub fn get_header(&self, name: &str) -> Option<&Vec<String>> {
        self.headers.get(&name.to_lowercase())
    }

    pub fn get_header_value(&self, name: &str) -> Option<&String> {
        self.headers
            .get(&name.to_lowercase())
            .and_then(|values| values.first())
    }

    pub fn get_body(&self) -> &str {
        &self.body
    }

    fn parse_path_and_query(path_and_query: &str) -> (String, HashMap<String, String>) {
        if let Some(question_pos) = path_and_query.find('?') {
            let path = path_and_query[..question_pos].to_string();
            let query_string = &path_and_query[question_pos + 1..];
            let query = Self::parse_query_string(query_string);
            (path, query)
        } else {
            (path_and_query.to_string(), HashMap::new())
        }
    }

    fn parse_query_string(query: &str) -> HashMap<String, String> {
        let mut params = HashMap::new();
        for pair in query.split('&') {
            if let Some(equals_pos) = pair.find('%') {
                // TODO: Implement URL decoding
                let key = pair[..equals_pos].to_string();
                let value = if equals_pos + 1 < pair.len() {
                    pair[equals_pos + 1..].to_string()
                } else {
                    String::new()
                };
                params.insert(key, value);
            } else if let Some(equals_pos) = pair.find('=') {
                let key = pair[..equals_pos].to_string();
                let value = if equals_pos + 1 < pair.len() {
                    pair[equals_pos + 1..].to_string()
                } else {
                    String::new()
                };
                params.insert(key, value);
            } else if !pair.is_empty() {
                params.insert(pair.to_string(), String::new());
            }
        }
        params
    }

    fn extract_content_length_from_map(headers: &HashMap<String, Vec<String>>) -> Option<usize> {
        headers
            .get("content-length")
            .and_then(|values| values.first())
            .and_then(|value| value.trim().parse().ok())
    }

    fn is_chunked_headers(headers: &HashMap<String, Vec<String>>) -> bool {
        headers
            .get("transfer-encoding")
            .and_then(|values| values.first())
            .map(|encoding| encoding.to_lowercase().contains("chunked"))
            .unwrap_or(false)
    }

    pub fn get_content_type(&self) -> Option<String> {
        self.get_header_value("content-type")
            .map(|value| value.split(';').next().unwrap_or("").trim().to_lowercase())
    }

    pub fn get_content_type_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        if let Some(content_type) = self.get_header_value("content-type") {
            for part in content_type.split(';').skip(1) {
                if let Some(equals_pos) = part.find('=') {
                    let key = part[..equals_pos].trim().to_lowercase();
                    let value = part[equals_pos + 1..].trim().trim_matches('"').to_string();
                    params.insert(key, value);
                }
            }
        }
        params
    }

    pub fn get_charset(&self) -> Option<String> {
        self.get_content_type_params().get("charset").cloned()
    }

    pub fn get_cookies(&self) -> HashMap<String, String> {
        let mut cookies = HashMap::new();
        if let Some(cookie_headers) = self.get_header("cookie") {
            for cookie_header in cookie_headers {
                for cookie_pair in cookie_header.split(';') {
                    let cookie_pair = cookie_pair.trim();
                    if let Some(equals_pos) = cookie_pair.find('=') {
                        let name = cookie_pair[..equals_pos].trim().to_string();
                        let value = cookie_pair[equals_pos + 1..].trim().to_string();
                        cookies.insert(name, value);
                    }
                }
            }
        }
        cookies
    }

    pub fn get_cookie(&self, name: &str) -> Option<String> {
        self.get_cookies().get(name).cloned()
    }

    pub fn is_form_data(&self) -> bool {
        self.get_content_type()
            .map(|ct| ct.contains("application/x-www-form-urlencoded"))
            .unwrap_or(false)
    }

    pub fn get_form_data(&self) -> HashMap<String, String> {
        if self.is_form_data() {
            Self::parse_query_string(&self.body)
        } else {
            HashMap::new()
        }
    }

    pub fn get_form_field(&self, field_name: &str) -> Option<String> {
        self.get_form_data().get(field_name).cloned()
    }

    pub fn is_json(&self) -> bool {
        self.get_content_type()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    pub fn get_json(&self) -> Option<serde_json::Value> {
        if self.is_json() {
            serde_json::from_str(&self.body).ok()
        } else {
            None
        }
    }

    pub fn get_json_field(&self, field: &str) -> Option<serde_json::Value> {
        self.get_json().and_then(|json| json.get(field).cloned())
    }

    pub fn is_chunked(&self) -> bool {
        self.get_header_value("transfer-encoding")
            .map(|encoding| encoding.to_lowercase().contains("chunked"))
            .unwrap_or(false)
    }

    fn parse_chunked_body(reader: &mut BufReader<TcpStream>, max_body_size: usize) -> String {
        let mut body = String::new();
        let mut total_size = 0;

        loop {
            // Read chunk size line
            let mut chunk_size_line = String::new();
            if reader.read_line(&mut chunk_size_line).unwrap() == 0 {
                break;
            }

            // Parse chunk size (hexadecimal)
            let chunk_size_str = chunk_size_line.trim();
            if chunk_size_str.is_empty() {
                continue;
            }

            let chunk_size =
                match usize::from_str_radix(chunk_size_str.split(';').next().unwrap_or("0"), 16) {
                    Ok(size) => size,
                    Err(_) => break,
                };

            if chunk_size == 0 {
                // End of chunks, read trailer headers if any
                let mut trailer_line = String::new();
                while reader.read_line(&mut trailer_line).unwrap() > 0 {
                    if trailer_line.trim().is_empty() {
                        break;
                    }
                    trailer_line.clear();
                }
                break;
            }

            if total_size + chunk_size > max_body_size {
                eprintln!(
                    "Warning: Chunked body exceeds limit of {} bytes",
                    max_body_size
                );
                // Read and discard remaining chunks
                let mut discard = vec![0u8; chunk_size];
                let _ = reader.read_exact(&mut discard);
                let mut crlf = [0u8; 2];
                let _ = reader.read_exact(&mut crlf);
                continue;
            }

            // Read chunk data
            let mut chunk_data = vec![0u8; chunk_size];
            reader.read_exact(&mut chunk_data).unwrap();
            body.push_str(&String::from_utf8_lossy(&chunk_data));

            total_size += chunk_size;

            // Read CRLF after chunk data
            let mut crlf = [0u8; 2];
            reader.read_exact(&mut crlf).unwrap();
        }

        body
    }
}

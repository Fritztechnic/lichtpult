//! Browser-Zugang zum Kern: `/api/<methode>` (POST) und die gebaute Oberfläche aus `ui/dist`.
//! Lauscht nur auf localhost. Dient der Entwicklung und Tests der Oberfläche.

use tiny_http::{Header, Response, Server};

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        _ => "application/octet-stream",
    }
}

fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8787".into());
    let dist = std::path::PathBuf::from(std::env::var("LICHTPULT_DIST").unwrap_or_else(|_| "ui/dist".into()));
    let core = lichtpult::api::Core::start();
    let server = Server::http(format!("127.0.0.1:{port}")).expect("bind");
    eprintln!("Lichtpult server on http://127.0.0.1:{port}");
    for mut req in server.incoming_requests() {
        let url = req.url().split('?').next().unwrap_or("/").to_string();
        let resp = if let Some(m) = url.strip_prefix("/api/") {
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);
            let (code, text) = match core.call(m, &body) {
                Ok(t) => (200, t),
                Err(e) => (400, e),
            };
            Response::from_string(text).with_status_code(code).with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
        } else {
            let rel = url.trim_start_matches('/');
            let rel = if rel.is_empty() || rel.contains("..") { "index.html" } else { rel };
            let file = std::fs::read(dist.join(rel)).or_else(|_| std::fs::read(dist.join("index.html")));
            match file {
                Ok(b) => Response::from_data(b).with_header(Header::from_bytes("Content-Type", mime(rel)).unwrap()).with_header(Header::from_bytes("Cache-Control", "no-cache").unwrap()),
                Err(_) => Response::from_string("not found").with_status_code(404),
            }
        };
        let _ = req.respond(resp);
    }
}

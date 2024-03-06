use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;
use esp_idf_svc::io::EspIOError;
use std::sync::{Arc, Mutex};

use crate::metrics::Metrics;

pub(crate) fn server(m: Arc<Mutex<Metrics>>) -> Result<EspHttpServer<'static>, EspIOError> {
    let mut httpserver = EspHttpServer::new(&Default::default())?;

    httpserver.fn_handler("/", Method::Get, move |req| {
        // Can't get `req.content_len()` to work, the Headers trait doesnt seem to work
        let mut resp = req.into_response(200, None, &[("content-type", "text/plain")])?;

        for ln in m.lock().unwrap().serialize() {
            resp.write(ln.as_bytes())?;
            resp.write(b"\n")?;
        }
        Ok::<(), EspIOError>(())
    })?;

    Ok(httpserver)
}

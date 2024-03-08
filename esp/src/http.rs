use crate::{clock_face::ClockFace, metrics::Metrics};
use esp_idf_svc::http::server::{Connection, EspHttpServer, Response};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::ErrorType;
use esp_idf_svc::io::EspIOError;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Deserialize, Debug)]
struct Form {
    timezone: String,
}

fn index<T: Connection>(resp: &mut Response<T>, c: Arc<Mutex<ClockFace>>) -> Result<(), EspIOError>
where
    EspIOError: From<<T as ErrorType>::Error>,
{
    let cf = c.lock().unwrap();
    let tz = cf.current_tz();
    resp.write(format!("current time is {}, timezone is {}, avail=", cf.now(), tz).as_bytes())?;
    resp.write(b"<form method=post>")?;
    resp.write(b"<select name='timezone'>")?;
    for ln in cf.avail_tz() {
        resp.write(b"<option>")?;
        resp.write(ln.to_string().as_bytes())?;
        resp.write(b"</option>")?;
    }
    resp.write(b"</select>")?;
    resp.write(r#"<input type="submit" value="Change">"#.as_bytes())?;
    resp.write(b"</form>")?;
    Ok::<(), EspIOError>(())
}
pub(crate) fn server(
    m: Arc<Mutex<Metrics>>,
    c: Arc<Mutex<ClockFace>>,
) -> Result<EspHttpServer<'static>, EspIOError> {
    let mut httpserver = EspHttpServer::new(&Default::default())?;

    let c1 = c.clone();
    let c2 = c.clone();
    httpserver.fn_handler("/", Method::Post, move |mut req| {
        let mut buf: Vec<u8> = vec![0; 64];
        req.read(&mut buf)?;
        let str_repr = std::str::from_utf8(&buf)
            .unwrap()
            .trim_end_matches(char::from(0));
        let f: Form = serde_urlencoded::from_str::<Form>(&str_repr).unwrap();
        println!("Updating timezone to {:?}", f);
        c1.lock().unwrap().set_tz(&f.timezone);

        let mut resp = req.into_response(200, None, &[("content-type", "text/html")])?;
        index(&mut resp, c1.clone())
    })?;

    httpserver.fn_handler("/", Method::Get, move |req| {
        let mut resp = req.into_response(200, None, &[("content-type", "text/html")])?;
        index(&mut resp, c2.clone())
    })?;

    httpserver.fn_handler("/metrics", Method::Get, move |req| {
        let mut resp = req.into_response(200, None, &[("content-type", "text/plain")])?;

        for ln in m.lock().unwrap().serialize() {
            resp.write(ln.as_bytes())?;
            resp.write(b"\n")?;
        }
        Ok::<(), EspIOError>(())
    })?;

    Ok(httpserver)
}

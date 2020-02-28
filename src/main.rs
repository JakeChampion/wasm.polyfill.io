extern crate error_chain;
extern crate querystring;

use fastly::http::{HeaderValue, Method, StatusCode, Uri, Version};
use fastly::{downstream_request, Body, Error, Request, RequestExt, Response, ResponseExt};
use std::result::Result;
const PASS: i32 = -1;

fn handle_request(mut req: Request<Body>) -> Result<Response<Body>, Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/v4/polyfill.min.js") => {
            let (parts, body) = req.into_parts();
            let mut request = Request::from_parts(parts, body);

            let uri = "https://polyfill.io/v3/normalizeUa".parse::<Uri>();

            match uri {
                Ok(u) => *request.uri_mut() = u,
                Err(e) => return Result::Err(Error::new(e)),
            }

            let headers = request.headers_mut();
            headers.insert("HOST", HeaderValue::from_static("polyfill.io"));

            let norm_resp = request.send("polyfill", PASS);

            let norm_resp = match norm_resp {
                Ok(resp) => resp,
                Err(e) => return Result::Err(e),
            };

            let normalized_ua = norm_resp.headers().get("Normalized-User-Agent");

            let normalized_ua = match normalized_ua {
                Some(ua) => (ua.clone()),
                None => {
                    return Result::Err(Error::msg("Normalized-User-Agent header did not exist"))
                }
            };

            let body = match Body::new() {
                Ok(b) => b,
                Err(e) => return Result::Err(e),
            };

            let mut bereq = Request::new(body);
            *bereq.method_mut() = Method::GET;
            *bereq.version_mut() = Version::HTTP_11;

            let uri = "https://polyfill.io/v3/polyfill.min.js".parse::<Uri>();

            match uri {
                Ok(u) => *bereq.uri_mut() = u,
                Err(e) => return Result::Err(Error::new(e)),
            }

            let headers = bereq.headers_mut();

            headers.insert("HOST", HeaderValue::from_static("polyfill.io"));

            headers.insert("User-Agent", normalized_ua);

            bereq.send("polyfill", PASS)
        }
        _ => {
            let headers = req.headers_mut();
            headers.insert("HOST", HeaderValue::from_static("polyfill.io"));
            req.send("polyfill", PASS)
        }
    }
}

fn main() -> Result<(), Error> {
    let req = downstream_request()?;
    let debug = req.headers().contains_key("Fastly-Debug");
    match handle_request(req) {
        Ok(resp) => {
            resp.send_downstream()?;
        }
        Err(e) => {
            let mut resp;
            if debug {
                resp = Response::new(Vec::from(e.to_string()));
            } else {
                resp = Response::new(Vec::from(""));
            }
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            resp.send_downstream()?;
        }
    }
    Ok(())
}

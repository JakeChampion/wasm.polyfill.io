#[macro_use]
extern crate lazy_static;
extern crate error_chain;
extern crate querystring;
use fastly::http::HeaderValue;
use fastly::http::Method;
use fastly::{downstream_request, uap_parse, Error, Request, Response, ResponseExt};
use regex::Regex;
const PASS: i32 = -1;

pub fn normalise(ua: &str) -> (String, String, String, String) {
    let mut useragent = String::from(ua);
    let mut family = "other".to_string().to_string();
    let mut major = "0".to_string();
    let mut minor = "0".to_string();
    let mut patch = "0".to_string();

    if useragent.is_empty() {
        return (family, major, minor, patch);
    } else {
        lazy_static! {
            static ref NORMALISED: Regex = Regex::new(
                r"(?ix)(?P<family>^\w+)/(?P<major>\d+)(?:\.(?P<minor>\d+)(?:\.(?P<patch>\d+))?)?$"
            )
            .unwrap();
        }
        lazy_static! {
            static ref GSA: Regex = Regex::new(r"(?i)/ GSA/[\d\.]+").unwrap();
        }
        lazy_static! {
            static ref INSTAGRAM: Regex = Regex::new(r"(?i) Instagram [\d\.]+").unwrap();
        }
        lazy_static! {
            static ref WEBPAGETEST: Regex = Regex::new(r"(?i) PTST/[\d\.]+").unwrap();
        }
        lazy_static! {
            static ref WATERFOX: Regex = Regex::new(r"(?i) Waterfox/[\d\.]+").unwrap();
        }
        lazy_static! {
            static ref GOANNA: Regex = Regex::new(r"(?i) Goanna/[\d\.]+").unwrap();
        }
        lazy_static! {
            static ref PALEMOON: Regex = Regex::new(r"(?i) PaleMoon/[\d\.]+").unwrap();
        }
        lazy_static! {
            static ref YANDEX: Regex = Regex::new(r"(?i)(YaBrowser)/(\d+\.)+\d+ ").unwrap();
        }
        lazy_static! {
            static ref CROSSWALK: Regex =
                Regex::new(r"(?i) (Crosswalk)/(\d+)\.(\d+)\.(\d+)\.(\d+)").unwrap();
        }
        lazy_static! {
            static ref CHROME_AND_OPERA_IOS: Regex =
                Regex::new(r"(?i)((CriOS|OPiOS)/(\d+)\.(\d+)\.(\d+)\.(\d+)|(FxiOS/(\d+)\.(\d+)))")
                    .unwrap();
        }
        lazy_static! {
            static ref VIVALDI: Regex = Regex::new(r"(?i) vivaldi/[\d\.]+\d+").unwrap();
        }
        lazy_static! {
            static ref FACEBOOK: Regex =
                Regex::new(r"(?i) \[(FB_IAB|FBAN|FBIOS|FB4A)/[^\]]+\]").unwrap();
        }
        lazy_static! {
            static ref ELECTRON: Regex = Regex::new(r"(?i) Electron/[\d\.]+\d+").unwrap();
        }
        lazy_static! {
            static ref EDGE_CHROME: Regex = Regex::new(r"(?i) Edg/[\d\.]+\d+").unwrap();
        }
        if NORMALISED.is_match(&useragent) {
            let captures = NORMALISED.captures(&useragent).unwrap();
            return (
                captures
                    .name("family")
                    .map_or("other".to_string(), |m| m.as_str().to_lowercase()),
                captures.name("major").unwrap().as_str().to_string(),
                captures
                    .name("minor")
                    .map_or(minor, |m| m.as_str().to_string()),
                patch,
            );
        } else {
            if GSA.is_match(&useragent) {
                // Google Search iOS app should be detected as the underlying browser, which is safari on iOS
                useragent = GSA.replace_all(&useragent, "").into_owned();
            } else if INSTAGRAM.is_match(&useragent) {
                // Instagram should be detected as the underlying browser, which is safari on ios
                useragent = INSTAGRAM.replace_all(&useragent, "").into_owned();
            } else if WEBPAGETEST.is_match(&useragent) {
                // WebPageTest is not a real browser, remove the token to find the underlying browser
                useragent = WEBPAGETEST.replace_all(&useragent, "").into_owned();
            } else if WATERFOX.is_match(&useragent) {
                // Waterfox is a Firefox fork, we can remove the Waterfox identifiers and parse the result as Firefox
                useragent = WATERFOX.replace_all(&useragent, "").into_owned();
            } else if GOANNA.is_match(&useragent) {
                // Pale Moon has a Firefox-compat UA string, we can remove the Pale Moon and Goanna identifiers and parse the result as Firefox
                useragent = GOANNA.replace_all(&useragent, "").into_owned();
            } else if PALEMOON.is_match(&useragent) {
                // Pale Moon has a Firefox-compat UA string, we can remove the Pale Moon and Goanna identifiers and parse the result as Firefox
                useragent = PALEMOON.replace_all(&useragent, "").into_owned();
            } else if YANDEX.is_match(&useragent) {
                // Yandex browser is recognised by UA module but is actually Chromium under the hood, so better to remove the Yandex identifier and get the UA module to detect it as Chrome
                useragent = YANDEX.replace_all(&useragent, "").into_owned();
            } else if CROSSWALK.is_match(&useragent) {
                // Crosswalk browser is recognised by UA module but is actually Chromium under the hood, so better to remove the identifier and get the UA module to detect it as Chrome
                useragent = CROSSWALK.replace_all(&useragent, "").into_owned();
            } else if CHROME_AND_OPERA_IOS.is_match(&useragent) {
                // Chrome and Opera on iOS uses a UIWebView of the underlying platform to render content. By stripping the CriOS or OPiOS strings, the useragent parser will alias the user agent to ios_saf for the UIWebView, which is closer to the actual renderer
                useragent = CHROME_AND_OPERA_IOS
                    .replace_all(&useragent, "")
                    .into_owned();
            } else if VIVALDI.is_match(&useragent) {
                // Vivaldi browser is recognised by UA module but is actually identical to Chrome, so the best way to get accurate targeting is to remove the vivaldi token from the UA
                useragent = VIVALDI.replace_all(&useragent, "").into_owned();
            } else if FACEBOOK.is_match(&useragent) {
                // Facebook in-app browser `[FBAN/.....]` or `[FB_IAB/.....]` (see https://github.com/Financial-Times/polyfill-service/issues/990)
                useragent = FACEBOOK.replace_all(&useragent, "").into_owned();
            } else if ELECTRON.is_match(&useragent) {
                // Electron/X.Y.Z` (see https://github.com/Financial-Times/polyfill-service/issues/1129)
                useragent = ELECTRON.replace_all(&useragent, "").into_owned();
            } else if EDGE_CHROME.is_match(&useragent) {
                // Chromium-based Edge
                useragent = EDGE_CHROME.replace_all(&useragent, "").into_owned();
            }

            let u = uap_parse(&useragent).unwrap();
            let s = uap_parse(&useragent).unwrap();
            let t = uap_parse(&useragent).unwrap();

            family = u.0.to_lowercase();
            major = s.1.unwrap_or(major);
            minor = t.2.unwrap_or(minor);
        }
        if family == "blackberry webkit" {
            family = "bb".to_string();
        }
        if family == "blackberry" {
            family = "bb".to_string();
        }
        if family == "pale moon (firefox variant)" {
            family = "firefox".to_string();
        }
        if family == "pale moon" {
            family = "firefox".to_string();
        }
        if family == "firefox mobile" {
            family = "firefox_mob".to_string();
        }
        if family == "firefox namoroka" {
            family = "firefox".to_string();
        }
        if family == "firefox shiretoko" {
            family = "firefox".to_string();
        }
        if family == "firefox minefield" {
            family = "firefox".to_string();
        }
        if family == "firefox alpha" {
            family = "firefox".to_string();
        }
        if family == "firefox beta" {
            family = "firefox".to_string();
        }
        if family == "microb" {
            family = "firefox".to_string();
        }
        if family == "mozilladeveloperpreview" {
            family = "firefox".to_string();
        }
        if family == "iceweasel" {
            family = "firefox".to_string();
        }
        if family == "opera tablet" {
            family = "opera".to_string();
        }
        if family == "opera mobile" {
            family = "op_mob".to_string();
        }
        if family == "opera mini" {
            family = "op_mini".to_string();
        }
        if family == "chrome mobile webview" {
            family = "chrome".to_string();
        }
        if family == "chrome mobile" {
            family = "chrome".to_string();
        }
        if family == "chrome frame" {
            family = "chrome".to_string();
        }
        if family == "chromium" {
            family = "chrome".to_string();
        }
        if family == "headlesschrome" {
            family = "chrome".to_string();
        }
        if family == "ie mobile" {
            family = "ie_mob".to_string();
        }
        if family == "ie large screen" {
            family = "ie".to_string();
        }
        if family == "internet explorer" {
            family = "ie".to_string();
        }
        if family == "edge mobile" {
            family = "edge_mob".to_string();
        }
        if family == "uc browser" {
            if major == "9" && minor == "9" {
                family = "ie".to_string();
                major = "10".to_string();
                minor = "0".to_string();
            }
        }
        if family == "chrome mobile ios" {
            family = "ios_chr".to_string();
        }
        if family == "mobile safari" {
            family = "ios_saf".to_string();
        }
        if family == "iphone" {
            family = "ios_saf".to_string();
        }
        if family == "iphone simulator" {
            family = "ios_saf".to_string();
        }
        if family == "mobile safari uiwebview" {
            family = "ios_saf".to_string();
        }
        if family == "mobile safari ui/wkwebview" {
            family = "ios_saf".to_string();
        }
        if family == "samsung internet" {
            family = "samsung_mob".to_string();
        }
        if family == "phantomjs" {
            family = "safari".to_string();
            major = "5".to_string();
            minor = "0".to_string();
        }
        if family == "opera" {
            if major == "20" {
                family = "chrome".to_string();
                major = "33".to_string();
                minor = "0".to_string();
            }
            if major == "21" {
                family = "chrome".to_string();
                major = "34".to_string();
                minor = "0".to_string();
            }
            if major == "22" {
                family = "chrome".to_string();
                major = "35".to_string();
                minor = "0".to_string();
            }
            if major == "23" {
                family = "chrome".to_string();
                major = "36".to_string();
                minor = "0".to_string();
            }
            if major == "24" {
                family = "chrome".to_string();
                major = "37".to_string();
                minor = "0".to_string();
            }
            if major == "25" {
                family = "chrome".to_string();
                major = "38".to_string();
                minor = "0".to_string();
            }
            if major == "26" {
                family = "chrome".to_string();
                major = "39".to_string();
                minor = "0".to_string();
            }
            if major == "27" {
                family = "chrome".to_string();
                major = "40".to_string();
                minor = "0".to_string();
            }
            if major == "28" {
                family = "chrome".to_string();
                major = "41".to_string();
                minor = "0".to_string();
            }
            if major == "29" {
                family = "chrome".to_string();
                major = "42".to_string();
                minor = "0".to_string();
            }
            if major == "30" {
                family = "chrome".to_string();
                major = "43".to_string();
                minor = "0".to_string();
            }
            if major == "31" {
                family = "chrome".to_string();
                major = "44".to_string();
                minor = "0".to_string();
            }
            if major == "32" {
                family = "chrome".to_string();
                major = "45".to_string();
                minor = "0".to_string();
            }
            if major == "33" {
                family = "chrome".to_string();
                major = "46".to_string();
                minor = "0".to_string();
            }
            if major == "34" {
                family = "chrome".to_string();
                major = "47".to_string();
                minor = "0".to_string();
            }
            if major == "35" {
                family = "chrome".to_string();
                major = "48".to_string();
                minor = "0".to_string();
            }
            if major == "36" {
                family = "chrome".to_string();
                major = "49".to_string();
                minor = "0".to_string();
            }
            if major == "37" {
                family = "chrome".to_string();
                major = "50".to_string();
                minor = "0".to_string();
            }
            if major == "38" {
                family = "chrome".to_string();
                major = "51".to_string();
                minor = "0".to_string();
            }
            if major == "39" {
                family = "chrome".to_string();
                major = "52".to_string();
                minor = "0".to_string();
            }
            if major == "40" {
                family = "chrome".to_string();
                major = "53".to_string();
                minor = "0".to_string();
            }
            if major == "41" {
                family = "chrome".to_string();
                major = "54".to_string();
                minor = "0".to_string();
            }
            if major == "42" {
                family = "chrome".to_string();
                major = "55".to_string();
                minor = "0".to_string();
            }
            if major == "43" {
                family = "chrome".to_string();
                major = "56".to_string();
                minor = "0".to_string();
            }
            if major == "44" {
                family = "chrome".to_string();
                major = "57".to_string();
                minor = "0".to_string();
            }
            if major == "45" {
                family = "chrome".to_string();
                major = "58".to_string();
                minor = "0".to_string();
            }
            if major == "46" {
                family = "chrome".to_string();
                major = "59".to_string();
                minor = "0".to_string();
            }
            if major == "47" {
                family = "chrome".to_string();
                major = "60".to_string();
                minor = "0".to_string();
            }
        }
        if family == "googlebot" {
            if major == "2" && minor == "1" {
                family = "chrome".to_string();
                major = "41".to_string();
                minor = "0".to_string();
            }
        }
        if false
            || family == "edge"
            || family == "edge_mob"
            || (family == "ie" && major.parse::<i32>().unwrap() >= 8)
            || (family == "ie_mob" && major.parse::<i32>().unwrap() >= 11)
            || (family == "chrome" && major.parse::<i32>().unwrap() >= 29)
            || (family == "safari" && major.parse::<i32>().unwrap() >= 9)
            || (family == "ios_saf" && major.parse::<i32>().unwrap() >= 9)
            || (family == "ios_chr" && major.parse::<i32>().unwrap() >= 9)
            || (family == "firefox" && major.parse::<i32>().unwrap() >= 38)
            || (family == "firefox_mob" && major.parse::<i32>().unwrap() >= 38)
            || (family == "android"
                && [&major, ".", &minor].concat().parse::<f32>().unwrap() >= 4.3)
            || (family == "opera" && major.parse::<i32>().unwrap() >= 33)
            || (family == "op_mob" && major.parse::<i32>().unwrap() >= 10)
            || (family == "op_mini" && major.parse::<i32>().unwrap() >= 5)
            || (family == "bb" && major.parse::<i32>().unwrap() >= 6)
            || (family == "samsung_mob" && major.parse::<i32>().unwrap() >= 4)
        {
        } else {
            family = "other".to_string();
            major = "0".to_string();
            minor = "0".to_string();
            patch = "0".to_string();
        }

        return (family, major, minor, patch);
    }
}

fn app(mut req: Request<fastly::body::Body>) -> Result<Response<fastly::body::Body>, Error> {
    let headers = req.headers();
    let resp = match (req.method(), req.uri().path()) {
        (&Method::GET, "/v4/polyfill.min.js") => {
            let ua = headers.get("user-agent").unwrap().to_str().unwrap();
            let (family, major, minor, patch) = normalise(ua);
            let normalized_ua = format!("{}/{}.{}.{}", family, major, minor, patch);
            let (mut parts, body) = req.into_parts();
            parts.method = Method::GET;
            parts
                .headers
                .insert(
                    "Host",
                    HeaderValue::from_str("polyfill.io").expect("set Host header"),
                )
                .unwrap();
            let mut qs = querystring::querify(parts.uri.query().unwrap_or("?"));
            // if ua querystring param is set, override it
            match qs.iter().position(|v| v.0 == "ua") {
                Some(p) => {
                    qs.remove(p);
                    qs.push(("ua", &normalized_ua));
                }
                None => {
                    qs.push(("ua", &normalized_ua));
                }
            }
            let backend = fastly::backend::Backend::from_str("polyfill");
            let path_and_query =
                format!("/v3/polyfill.min.js?{}", querystring::stringify(qs)).to_owned();
            let uri = fastly::http::uri::Uri::builder()
                .scheme("https")
                .authority("polyfill.io")
                .path_and_query(path_and_query.as_str())
                .build()
                .unwrap();
            parts.uri = uri;
            let bereq = Request::from_parts(parts, body);
            let mut beresp = backend.send(bereq, PASS).expect("malform backend request");
            let beresp_headers = beresp.headers_mut();
            beresp_headers
                .insert(
                    "normalized_ua",
                    HeaderValue::from_str(&normalized_ua).expect("malformed normalized_ua"),
                )
                .unwrap();
            beresp_headers
                .insert(
                    "uri",
                    HeaderValue::from_str(path_and_query.as_str()).expect("malformed uri"),
                )
                .unwrap();
            beresp
        }
        _ => {
            let backend = fastly::backend::Backend::from_str("polyfill");
            req.headers_mut()
                .insert(
                    "HOST",
                    HeaderValue::from_str("polyfill.io").expect("set Host header"),
                )
                .unwrap();
            let beresp = backend.send(req, PASS).expect("malform backend request");
            beresp
        }
    };

    Ok(resp)
}

fn main() -> Result<(), Error> {
    let req = downstream_request()?;
    match app(req) {
        Ok(resp) => {
            resp.send_downstream()?;
        }
        Err(e) => {
            let mut resp = Response::new(Vec::from(e.msg));
            *resp.status_mut() = fastly::http::StatusCode::INTERNAL_SERVER_ERROR;
            resp.send_downstream()?;
        }
    }
    Ok(())
}

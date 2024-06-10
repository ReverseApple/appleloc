use protobuf::Message;
use thiserror::Error;

use crate::constants::{API_BASE, COORD_ERROR, H_IDENTIFIER, H_LOCALE, H_VERSION, USER_AGENT};
use crate::Error::{BssidNotFound, QueryError};
use crate::gsloc_proto::request::RequestWifi;
use crate::gsloc_proto::Response;

mod constants;
mod gsloc_proto;

macro_rules! string {
    ($ss:expr) => {String::from_utf8($ss).unwrap()};
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("The BSSID \"{0}\" was not found.")]
    BssidNotFound(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

#[inline(always)]
fn be_i16(num: i16) -> Vec<u8> {
    num.to_be_bytes().into()
}

#[inline(always)]
fn coord(coord: i64) -> f64 {
    coord as f64 * 1e-8
}

fn payload_header() -> Vec<u8> {
    const NUL_SQH: &str = "\x00\x01";
    const NUL_NUL: &str = "\x00\x00";

    let locale_length = be_i16(H_LOCALE.len() as i16);
    let identifier_length = be_i16(H_IDENTIFIER.len() as i16);
    let version_length = be_i16(H_VERSION.len() as i16);

    let result = format!(
        "{}{}{}{}{}{}{}{}{}{}",
        NUL_SQH,
        string!(locale_length),
        H_LOCALE,
        string!(identifier_length),
        H_IDENTIFIER,
        string!(version_length),
        H_VERSION,
        NUL_NUL,
        NUL_SQH,
        NUL_NUL
    );

    result.into_bytes()
}

fn create_payload(bssids: &[&str], signal: i32, noise: i32) -> Vec<u8> {
    let wifis: Vec<RequestWifi> = bssids
        .to_vec()
        .iter()
        .map(|s| RequestWifi {
            mac: Some(s.to_string()),
            special_fields: Default::default(),
        })
        .collect();

    let request = gsloc_proto::Request {
        wifis,
        noise: Some(noise),
        signal: Some(signal),
        source: None,
        special_fields: Default::default(),
    };

    let mut serialized = request.write_to_bytes().unwrap();

    serialized.splice(..0, be_i16(serialized.len() as i16));
    serialized.splice(..0, payload_header());


    serialized
}

fn send(payload: &[u8]) -> Result<Response, Error> {
    let client = reqwest::blocking::Client::new();

    let http_res = client
        .post(API_BASE)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(payload.to_vec())
        .send()
        .map_err(|e| QueryError(e.to_string()))?;

    if http_res.status().is_server_error() || http_res.status().is_client_error() {
        panic!("HTTP ERROR: {}", http_res.status().as_u16())
    }

    let resp_bytes = http_res.bytes().unwrap();

    let mut response = Response::new();
    response
        .merge_from_bytes(&resp_bytes.to_vec().as_slice()[10..])
        .expect("Failed to parse response.");

    Ok(response)
}

pub fn basic_location(bssid: &str) -> Result<(f64, f64), Error> {
    let payload = create_payload(&[bssid], 100, 0);

    let response = send(&payload)?;

    if response.wifis.len() == 0 {
        return Err(BssidNotFound(bssid.to_string()));
    }

    let wifi_location = response.wifis[0].location.clone();

    if wifi_location.latitude.unwrap() as u64 == COORD_ERROR {
        return Err(BssidNotFound(bssid.to_string()));
    }

    let lat = coord(wifi_location.latitude.unwrap());
    let long = coord(wifi_location.longitude.unwrap());

    Ok((lat, long))
}

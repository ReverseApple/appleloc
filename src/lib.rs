use mac6::Mac;
use prost::Message;
use protobuf::Payload;
use thiserror::Error;

use crate::constants::{API_ENDPOINT, COORD_ERROR, USER_AGENT};
use crate::Error::{BssidNotFound, QueryError};

mod constants;

mod protobuf {
    include!(concat!(env!("OUT_DIR"), "/wloc.rs"));
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("The BSSID \"{0}\" was not found.")]
    BssidNotFound(String),

    #[error("Query error: {0}")]
    QueryError(String),
}

#[inline(always)]
fn coord(coord: i64) -> f64 {
    coord as f64 * 1e-8
}

fn create_payload(bssids: &[Mac]) -> Vec<u8> {
    let proto = protobuf::Payload {
        wifis: bssids
            .into_iter()
            .map(|bssid| protobuf::WiFi {
                bssid: bssid.to_string(),
                location: None,
            })
            .collect(),
    };

    let mut payload = Vec::new();

    // header
    payload.extend([0x00, 0x01, 0x00, 0x05]);
    payload.extend("en_US".as_bytes());
    payload.extend([0x00, 0x13]);
    payload.extend("com.apple.locationd".as_bytes());
    payload.extend([0x00, 0x0a]);
    payload.extend("17.5.21F79".as_bytes());
    payload.extend([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00]);

    proto.encode_length_delimited(&mut payload).unwrap();
    proto.encode(&mut payload).unwrap();

    payload
}

fn send(payload: &[u8]) -> Result<Payload, Error> {
    let client = reqwest::blocking::Client::new();

    let http_res = client
        .post(API_ENDPOINT)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(payload.to_vec())
        .send()
        .map_err(|e| QueryError(e.to_string()))?;

    if http_res.status().is_server_error() || http_res.status().is_client_error() {
        panic!("HTTP ERROR: {}", http_res.status().as_u16())
    }

    let resp_bytes = http_res.bytes().unwrap();

    let response =
        Payload::decode(&resp_bytes.to_vec().as_slice()[10..]).expect("Failed to parse response.");
    Ok(response)
}

pub fn basic_location(bssid: &str) -> Result<(f64, f64), Error> {
    let mac = bssid
        .parse()
        .map_err(|_| Error::QueryError("Invalid MAC address".to_string()))?;
    let payload = create_payload(&[mac]);

    let response = send(&payload)?;

    if response.wifis.len() == 0 {
        return Err(BssidNotFound(mac.to_string()));
    }

    let location = response.wifis[0]
        .location
        .expect("response must have a location value");
    if location.latitude as u64 == COORD_ERROR {
        return Err(BssidNotFound(mac.to_string()));
    }

    let lat = coord(location.latitude);
    let long = coord(location.longitude);

    Ok((lat, long))
}

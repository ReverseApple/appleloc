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

struct Location {
    latitude: f64,
    longitude: f64,
    accuracy: i64,
    altitude: i64,
    altitude_accuracy: i64,
}

impl Location {
    fn from_proto(proto: protobuf::Location) -> Option<Self> {
        let (lat, lon) = (proto.latitude, proto.longitude);
        if lat == -18000000000 {
            None
        } else {
            let (latitude, longitude) = (coord(lat), coord(lon));
            Some(Self {
                latitude,
                longitude,
                accuracy: proto.accuracy,
                altitude: proto.altitude,
                altitude_accuracy: proto.altitude_accuracy,
            })
        }
    }
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

fn send(payload: &[u8]) -> Result<Vec<(Mac, Option<Location>)>, Error> {
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

    Ok(response
        .wifis
        .into_iter()
        .map(|x| {
            // TODO: handle parsing failures gracefully
            let mac = x.bssid.parse().unwrap();
            let loc =
                Location::from_proto(x.location.expect("Response should include location field"));
            (mac, loc)
        })
        .collect())
}

pub fn basic_location(bssid: &str) -> Result<(f64, f64), Error> {
    let mac = bssid
        .parse()
        .map_err(|_| Error::QueryError("Invalid MAC address".to_string()))?;
    let payload = create_payload(&[mac]);

    let locs = send(&payload)?;

    if locs.len() == 0 {
        panic!("server did not respond with any data")
    }

    if let Some(loc) = &locs[0].1 {
        Ok((loc.latitude, loc.longitude))
    } else {
        Err(BssidNotFound(mac.to_string()))
    }
}

// can apparently also be https://iphone-services.apple.com/clls/wloc
pub(crate) const API_BASE: &str = "https://gs-loc.apple.com/clls/wloc";

pub(crate) const USER_AGENT: &str = "locationd/1756.1.15 CFNetwork/711.5.6 Darwin/14.0.0";

pub(crate) const COORD_ERROR: u64 = 18446744055709551616;

// payload header stuff below...

pub(crate) const H_LOCALE: &str = "en_US";

pub(crate) const H_IDENTIFIER: &str = "com.apple.locationd";

pub(crate) const H_VERSION: &str = "8.4.1.12H321";

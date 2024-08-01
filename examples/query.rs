use std::env;

use appleloc::basic_location;

fn main() {
    for arg in env::args().skip(1) {
        let res = basic_location(&arg);
        println!("{arg}: {res:?}");
    }
}

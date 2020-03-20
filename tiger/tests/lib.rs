#![no_std]
#[macro_use]
extern crate digest;
extern crate tiger;

use digest::dev::{digest_test, one_million_a};

new_test!(tiger_main, "tiger", tiger::Tiger, digest_test);

#[test]
fn tiger_1million_a() {
    let output = include_bytes!("data/one_million_a.bin");
    one_million_a::<tiger::Tiger>(output);
}

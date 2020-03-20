#![no_std]
#![feature(test)]
#[macro_use]
extern crate digest;
extern crate tiger;

bench!(tiger::Tiger);

//! The [Tiger][1] hash function.
//!
//! [1]: https://en.wikipedia.org/wiki/Tiger_(hash_function)

#![no_std]
#[macro_use] extern crate opaque_debug;
#[macro_use] extern crate digest;
extern crate block_buffer;
extern crate byte_tools;

use core::mem;
use core::num::Wrapping;

pub use digest::Digest;
use digest::{Input, BlockInput, FixedOutput, Reset};
use digest::generic_array::GenericArray;
use digest::generic_array::typenum::{U24, U64};

use byte_tools::{read_u64v_le, write_u64v_le};

use block_buffer::BlockBuffer;
use block_buffer::byteorder::LE;

#[macro_use]
mod macros;
mod consts;

use consts::*;

type BlockSize = U64;
type Block = GenericArray<u8, BlockSize>;

#[derive(Clone)]
pub struct Tiger {
    buffer: BlockBuffer<U64>,
    len: u64,
    state: TigerState,
}

#[derive(Clone)]
struct TigerState((u64, u64, u64));

const A: u64 = 0x0123456789ABCDEF;
const B: u64 = 0xFEDCBA9876543210;
const C: u64 = 0xF096A5B4C3B2E187;

impl TigerState {
    fn new() -> Self {
        TigerState((A, B, C))
    }

    fn process_block(&mut self, block: &Block) {
        let (a, b, c) = self.0;
        let (mut a, mut b, mut c) = (Wrapping(a), Wrapping(b), Wrapping(c));

        let mut data: [u64; 8] = [0; 8];
        read_u64v_le(&mut data, block);

        let mut data = unsafe { mem::transmute::<_, [Wrapping<u64>; 8]>(data) };

        compress!(data, a, b, c);

        self.0 = (a.0, b.0, c.0);
    }

    fn get(&self) -> (u64, u64, u64) {
        self.0
    }
}

impl Tiger {
    pub fn new() -> Self {
        Tiger {
            buffer: BlockBuffer::default(),
            len: 0,
            state: TigerState::new(),
        }
    }

    fn process_block(&mut self, input: &[u8]) {
        let self_state = &mut self.state;
        self.buffer.input(input,
                          |blk| self_state.process_block(blk));
    }

    fn finalize(&mut self) {
        let self_state = &mut self.state;
        self.buffer.len64_padding::<LE, _>(self.len, |blk| self_state.process_block(blk));
    }
}

impl Default for Tiger  {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockInput for Tiger {
    type BlockSize = U64;
}

impl Input for Tiger {
    fn input<B: AsRef<[u8]>>(&mut self, input: B) {
        let input = input.as_ref();
        self.process_block(input);
        self.len += (input.len() << 3) as u64;
    }
}

type Output = GenericArray<u8, U24>;

impl FixedOutput for Tiger {
    type OutputSize = U24;

    fn fixed_result(mut self) -> Output {
        self.finalize();

        let (a, b, c) = self.state.get();

        let mut output = Output::default();
        write_u64v_le(output.as_mut_slice(), &[a, b, c]);

        output
    }
}

impl Reset for Tiger {
    fn reset(&mut self) {
        self.state = TigerState((A, B, C));
        self.buffer.reset();
        self.len = 0;
    }
}

impl_opaque_debug!(Tiger);
impl_write!(Tiger);

#[cfg(test)]
mod tests {
    use super::*;

    use core::num::ParseIntError;

    fn hex_to_bytes(hex: &str) -> Result<Output, ParseIntError> {
        let mut bytes = [0; 24];
        for i in 0..hex.len()/2 {
            bytes[i] = u8::from_str_radix(&hex[2*i..2*i+2], 16)?;
        }
        Ok(bytes.into())
    }

    fn tiger_hash(input: &[u8]) -> Output {
        let mut hasher = Tiger::new();
        Input::input(&mut hasher, input);
        hasher.result()
    }

    #[test]
    fn basic_test() {
        let test_cases: &'static [(&'static [u8], &'static str)] = &[
            (b"",                                                                                 "4441BE75F6018773C206C22745374B924AA8313FEF919F41"),
            (b"a",                                                                                "67E6AE8E9E968999F70A23E72AEAA9251CBC7C78A7916636"),
            (b"abc",                                                                              "F68D7BC5AF4B43A06E048D7829560D4A9415658BB0B1F3BF"),
            (b"message digest",                                                                   "E29419A1B5FA259DE8005E7DE75078EA81A542EF2552462D"),
            (b"abcdefghijklmnopqrstuvwxyz",                                                       "F5B6B6A78C405C8547E91CD8624CB8BE83FC804A474488FD"),
            (b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",                         "A6737F3997E8FBB63D20D2DF88F86376B5FE2D5CE36646A9"),
            (b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",                   "EA9AB6228CEE7B51B77544FCA6066C8CBB5BBAE6319505CD"),
            (b"12345678901234567890123456789012345678901234567890123456789012345678901234567890", "D85278115329EBAA0EEC85ECDC5396FDA8AA3A5820942FFF"),
        ];

        for (i, &(input, expected_hex)) in test_cases.iter().enumerate() {
            let expected = hex_to_bytes(expected_hex).unwrap();
            let reached = tiger_hash(input);

            assert_eq!((i, &expected), (i, &reached));
        }
    }
}

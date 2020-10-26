#![feature(test)]
#![deny(warnings)]

extern crate test;

#[allow(deprecated)]
use asn1rs::io::buffer::legacy::*;
use test::Bencher;

macro_rules! bench_stuff {
    ($name_legacy: ident, $name_new: ident, $offset: expr, $pos: expr) => {
        #[bench]
        #[allow(deprecated)]
        fn $name_legacy(b: &mut Bencher) {
            b.iter(|| legacy_bit_buffer(SIZE_BITS, $offset, $pos));
            legacy_bit_buffer_with_check(SIZE_BITS, $offset, $pos)
        }

        #[bench]
        #[allow(deprecated)]
        fn $name_new(b: &mut Bencher) {
            b.iter(|| new_bit_buffer(SIZE_BITS, $offset, $pos));
            new_bit_buffer_with_check(SIZE_BITS, $offset, $pos)
        }
    };
}

bench_stuff!(legacy_offset_0_position_0, new_offset_0_position_0, 0, 0);
bench_stuff!(legacy_offset_3_position_0, new_offset_3_position_0, 3, 0);
bench_stuff!(legacy_offset_4_position_0, new_offset_4_position_0, 4, 0);
bench_stuff!(legacy_offset_7_position_0, new_offset_7_position_0, 7, 0);

bench_stuff!(legacy_offset_0_position_3, new_offset_0_position_3, 0, 3);
bench_stuff!(legacy_offset_3_position_3, new_offset_3_position_3, 3, 3);
bench_stuff!(legacy_offset_4_position_3, new_offset_4_position_3, 4, 3);
bench_stuff!(legacy_offset_7_position_3, new_offset_7_position_3, 7, 3);

bench_stuff!(legacy_offset_0_position_4, new_offset_0_position_4, 0, 4);
bench_stuff!(legacy_offset_3_position_4, new_offset_3_position_4, 3, 4);
bench_stuff!(legacy_offset_4_position_4, new_offset_4_position_4, 4, 4);
bench_stuff!(legacy_offset_7_position_4, new_offset_7_position_4, 7, 4);

bench_stuff!(legacy_offset_0_position_7, new_offset_0_position_7, 0, 7);
bench_stuff!(legacy_offset_3_position_7, new_offset_3_position_7, 3, 7);
bench_stuff!(legacy_offset_4_position_7, new_offset_4_position_7, 4, 7);
bench_stuff!(legacy_offset_7_position_7, new_offset_7_position_7, 7, 7);

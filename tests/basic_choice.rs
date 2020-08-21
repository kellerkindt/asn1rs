#![recursion_limit = "512"]

use asn1rs::prelude::*;
use asn1rs::syn::io::UperReader as NewUperReader;
use asn1rs::syn::io::UperWriter as NewUperWriter;

asn_to_rust!(
    r"BasicChoice DEFINITIONS AUTOMATIC TAGS ::=
    BEGIN
    
    Basic ::= CHOICE {
        abc UTF8String,
        def UTF8String,
        ghi INTEGER
    }
    
    Extensible ::= CHOICE {
        abc UTF8String,
        def INTEGER,
        ..., -- whatever reserved blubber comment
        ghi INTEGER,
        jkl Basic,
        mno UTF8String
    }
    
    MoreThan63Extensions ::= CHOICE {
        abc UTF8String,
        ..., -- whatever reserved blubber comment
        e00 INTEGER,
        e01 INTEGER,
        e02 INTEGER,
        e03 INTEGER,
        e04 INTEGER,
        e05 INTEGER,
        e06 INTEGER,
        e07 INTEGER,
        e08 INTEGER,
        e09 INTEGER,
        
        e10 INTEGER,
        e11 INTEGER,
        e12 INTEGER,
        e13 INTEGER,
        e14 INTEGER,
        e15 INTEGER,
        e16 INTEGER,
        e17 INTEGER,
        e18 INTEGER,
        e19 INTEGER,
        
        e20 INTEGER,
        e21 INTEGER,
        e22 INTEGER,
        e23 INTEGER,
        e24 INTEGER,
        e25 INTEGER,
        e26 INTEGER,
        e27 INTEGER,
        e28 INTEGER,
        e29 INTEGER,
        
        e30 INTEGER,
        e31 INTEGER,
        e32 INTEGER,
        e33 INTEGER,
        e34 INTEGER,
        e35 INTEGER,
        e36 INTEGER,
        e37 INTEGER,
        e38 INTEGER,
        e39 INTEGER,
        
        e40 INTEGER,
        e41 INTEGER,
        e42 INTEGER,
        e43 INTEGER,
        e44 INTEGER,
        e45 INTEGER,
        e46 INTEGER,
        e47 INTEGER,
        e48 INTEGER,
        e49 INTEGER,
        
        e50 INTEGER,
        e51 INTEGER,
        e52 INTEGER,
        e53 INTEGER,
        e54 INTEGER,
        e55 INTEGER,
        e56 INTEGER,
        e57 INTEGER,
        e58 INTEGER,
        e59 INTEGER,
        
        e60 INTEGER,
        e61 INTEGER,
        e62 INTEGER,
        e63 INTEGER,
        e64 INTEGER,
        e65 INTEGER,
        e66 INTEGER,
        e67 INTEGER,
        e68 INTEGER,
        e69 INTEGER,
        
        e70 INTEGER,
        e71 INTEGER,
        e72 INTEGER,
        e73 INTEGER,
        e74 INTEGER,
        e75 INTEGER,
        e76 INTEGER,
        e77 INTEGER,
        e78 INTEGER,
        e79 INTEGER,
        
        e80 INTEGER,
        e81 INTEGER,
        e82 INTEGER,
        e83 INTEGER,
        e84 INTEGER,
        e85 INTEGER,
        e86 INTEGER,
        e87 INTEGER,
        e88 INTEGER,
        e89 INTEGER,
        
        e90 INTEGER,
        e91 INTEGER,
        e92 INTEGER,
        e93 INTEGER,
        e94 INTEGER,
        e95 INTEGER,
        e96 INTEGER,
        e97 INTEGER,
        e98 INTEGER,
        e99 INTEGER,
        
        e100 INTEGER,
        e101 INTEGER,
        e102 INTEGER,
        e103 INTEGER,
        e104 INTEGER,
        e105 INTEGER,
        e106 INTEGER,
        e107 INTEGER,
        e108 INTEGER,
        e109 INTEGER,
        
        e110 INTEGER,
        e111 INTEGER,
        e112 INTEGER,
        e113 INTEGER,
        e114 INTEGER,
        e115 INTEGER,
        e116 INTEGER,
        e117 INTEGER,
        e118 INTEGER,
        e119 INTEGER,
        
        e120 INTEGER,
        e121 INTEGER,
        e122 INTEGER,
        e123 INTEGER,
        e124 INTEGER,
        e125 INTEGER,
        e126 INTEGER,
        e127 INTEGER,
        e128 INTEGER,
        e129 INTEGER,
        
        e130 INTEGER,
        e131 INTEGER,
        e132 INTEGER,
        e133 INTEGER,
        e134 INTEGER,
        e135 INTEGER,
        e136 INTEGER,
        e137 INTEGER,
        e138 INTEGER,
        e139 INTEGER,
        
        e140 INTEGER,
        e141 INTEGER,
        e142 INTEGER,
        e143 INTEGER,
        e144 INTEGER,
        e145 INTEGER,
        e146 INTEGER,
        e147 INTEGER,
        e148 INTEGER,
        e149 INTEGER,
        
        e150 INTEGER,
        e151 INTEGER,
        e152 INTEGER,
        e153 INTEGER,
        e154 INTEGER,
        e155 INTEGER,
        e156 INTEGER,
        e157 INTEGER,
        e158 INTEGER,
        e159 INTEGER,
        
        e160 INTEGER,
        e161 INTEGER,
        e162 INTEGER,
        e163 INTEGER,
        e164 INTEGER,
        e165 INTEGER,
        e166 INTEGER,
        e167 INTEGER,
        e168 INTEGER,
        e169 INTEGER,
        
        e170 INTEGER,
        e171 INTEGER,
        e172 INTEGER,
        e173 INTEGER,
        e174 INTEGER,
        e175 INTEGER,
        e176 INTEGER,
        e177 INTEGER,
        e178 INTEGER,
        e179 INTEGER,
        
        e180 INTEGER,
        e181 INTEGER,
        e182 INTEGER,
        e183 INTEGER,
        e184 INTEGER,
        e185 INTEGER,
        e186 INTEGER,
        e187 INTEGER,
        e188 INTEGER,
        e189 INTEGER,
        
        e190 INTEGER,
        e191 INTEGER,
        e192 INTEGER,
        e193 INTEGER,
        e194 INTEGER,
        e195 INTEGER,
        e196 INTEGER,
        e197 INTEGER,
        e198 INTEGER,
        e199 INTEGER,
        
        e200 INTEGER,
        e201 INTEGER,
        e202 INTEGER,
        e203 INTEGER,
        e204 INTEGER,
        e205 INTEGER,
        e206 INTEGER,
        e207 INTEGER,
        e208 INTEGER,
        e209 INTEGER,
        
        e210 INTEGER,
        e211 INTEGER,
        e212 INTEGER,
        e213 INTEGER,
        e214 INTEGER,
        e215 INTEGER,
        e216 INTEGER,
        e217 INTEGER,
        e218 INTEGER,
        e219 INTEGER,
        
        e220 INTEGER,
        e221 INTEGER,
        e222 INTEGER,
        e223 INTEGER,
        e224 INTEGER,
        e225 INTEGER,
        e226 INTEGER,
        e227 INTEGER,
        e228 INTEGER,
        e229 INTEGER,
        
        e230 INTEGER,
        e231 INTEGER,
        e232 INTEGER,
        e233 INTEGER,
        e234 INTEGER,
        e235 INTEGER,
        e236 INTEGER,
        e237 INTEGER,
        e238 INTEGER,
        e239 INTEGER,
        
        e240 INTEGER,
        e241 INTEGER,
        e242 INTEGER,
        e243 INTEGER,
        e244 INTEGER,
        e245 INTEGER,
        e246 INTEGER,
        e247 INTEGER,
        e248 INTEGER,
        e249 INTEGER,
        
        e250 INTEGER,
        e251 INTEGER,
        e252 INTEGER,
        e253 INTEGER,
        e254 INTEGER,
        e255 INTEGER,
        e256 INTEGER,
        e257 INTEGER,
        e258 INTEGER,
        e259 INTEGER,
        
        e260 INTEGER,
        e261 INTEGER,
        e262 INTEGER,
        e263 INTEGER,
        e264 INTEGER,
        e265 INTEGER,
        e266 INTEGER,
        e267 INTEGER,
        e268 INTEGER,
        e269 INTEGER,
        
        e270 INTEGER,
        e271 INTEGER,
        e272 INTEGER,
        e273 INTEGER,
        e274 INTEGER,
        e275 INTEGER,
        e276 INTEGER,
        e277 INTEGER,
        e278 INTEGER,
        e279 INTEGER
    }
    
    END"
);

fn serialize_uper(to_uper: &impl Writable) -> (usize, Vec<u8>) {
    let mut writer = NewUperWriter::default();
    writer.write(to_uper).unwrap();
    let bits = writer.bit_len();
    (bits, writer.into_bytes_vec())
}

fn deserialize_uper<T: Readable>(data: &[u8], bits: usize) -> T {
    let mut reader = NewUperReader::from_bits(data, bits);
    reader.read::<T>().unwrap()
}

fn serialize_and_deserialize_uper<T: Readable + Writable + std::fmt::Debug + PartialEq>(
    bits: usize,
    data: &[u8],
    uper: &T,
) {
    let serialized = serialize_uper(uper);
    assert_eq!((bits, data), (serialized.0, &serialized.1[..]));
    assert_eq!(uper, &deserialize_uper::<T>(data, bits));
}

#[test]
fn test_extensible_more_than_63_extensions_uper() {
    // from playground
    // this tests effectively
    //  - UperWriter::write_int_max_unsigned
    //  - UperReader::read_int_max_unsigned
    serialize_and_deserialize_uper(
        42,
        &[0xC0, 0x51, 0x40, 0x80, 0x40, 0x00],
        &MoreThan63Extensions::E69(0),
    );
    serialize_and_deserialize_uper(
        42,
        &[0xC0, 0x51, 0x40, 0x80, 0x45, 0x80],
        &MoreThan63Extensions::E69(22),
    );
    serialize_and_deserialize_uper(
        32,
        &[0x93, 0x02, 0x01, 0x16],
        &MoreThan63Extensions::E19(22),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e126() {
    serialize_and_deserialize_uper(
        5 * 8 + 2,
        &[0xC0, 0x5F, 0x80, 0x80, 0x5F, 0x80],
        &MoreThan63Extensions::E126(126),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e127() {
    serialize_and_deserialize_uper(
        5 * 8 + 2,
        &[0xC0, 0x5F, 0xC0, 0x80, 0x5F, 0xC0],
        &MoreThan63Extensions::E127(127),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e128() {
    serialize_and_deserialize_uper(
        6 * 8 + 2,
        &[0xC0, 0x60, 0x00, 0xC0, 0x80, 0x20, 0x00],
        &MoreThan63Extensions::E128(128),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e129() {
    serialize_and_deserialize_uper(
        6 * 8 + 2,
        &[0xC0, 0x60, 0x40, 0xC0, 0x80, 0x20, 0x40],
        &MoreThan63Extensions::E129(129),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e254() {
    serialize_and_deserialize_uper(
        6 * 8 + 2,
        &[0xC0, 0x7F, 0x80, 0xC0, 0x80, 0x3F, 0x80],
        &MoreThan63Extensions::E254(254),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e255() {
    serialize_and_deserialize_uper(
        6 * 8 + 2,
        &[0xC0, 0x7F, 0xC0, 0xC0, 0x80, 0x3F, 0xC0],
        &MoreThan63Extensions::E255(255),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e256() {
    serialize_and_deserialize_uper(
        7 * 8 + 2,
        &[0xC0, 0x80, 0x40, 0x00, 0xC0, 0x80, 0x40, 0x00],
        &MoreThan63Extensions::E256(256),
    );
}

#[test]
fn test_extensible_more_than_63_extensions_uper_e257() {
    serialize_and_deserialize_uper(
        7 * 8 + 2,
        &[0xC0, 0x80, 0x40, 0x40, 0xC0, 0x80, 0x40, 0x40],
        &MoreThan63Extensions::E257(257),
    );
}

#[test]
fn test_extensible_uper() {
    // https://asn1.io/asn1playground/
    // value Extensible ::=  abc { "" }
    serialize_and_deserialize_uper(10, &[0x00, 0x00], &Extensible::Abc(String::default()));
    serialize_and_deserialize_uper(
        106,
        &[
            0x03, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x15, 0xdb, 0xdc, 0x9b, 0x19, 0x08, 0x40,
        ],
        &Extensible::Abc("Hello World!".to_string()),
    );
    serialize_and_deserialize_uper(18, &[0x40, 0x40, 0x00], &Extensible::Def(0));
    serialize_and_deserialize_uper(26, &[0x40, 0x81, 0x4e, 0x40], &Extensible::Def(1337));

    // value Extensible ::=  ghi:0
    serialize_and_deserialize_uper(32, &[0x80_u8, 0x02, 0x01, 0x00], &Extensible::Ghi(0));

    // value Extensible ::=  ghi:27
    serialize_and_deserialize_uper(32, &[0x80_u8, 0x02, 0x01, 0x1B], &Extensible::Ghi(27));

    serialize_and_deserialize_uper(
        40,
        &[0x80_u8, 0x03, 0x02, 0x05, 0x39],
        &Extensible::Ghi(1337),
    );

    serialize_and_deserialize_uper(
        120,
        &[
            0x82, 0x0d, 0x0c, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x61, 0x67, 0x61, 0x69, 0x6e,
            0x21,
        ],
        &Extensible::Mno("Hello again!".to_string()),
    );
}

#[test]
pub fn test_basic_uper() {
    serialize_and_deserialize_uper(
        106,
        &[
            0x03, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x15, 0xdb, 0xdc, 0x9b, 0x19, 0x08, 0x40,
        ],
        &Basic::Abc("Hello World!".to_string()),
    );
    serialize_and_deserialize_uper(
        106,
        &[
            0x43, 0x12, 0x19, 0x5b, 0x1b, 0x1b, 0xc8, 0x18, 0x59, 0xd8, 0x5a, 0x5b, 0x88, 0x40,
        ],
        &Basic::Def("Hello again!".to_string()),
    );
    serialize_and_deserialize_uper(26, &[0x80, 0x81, 0x4e, 0x40], &Basic::Ghi(1337));
}

#[test]
fn test_extensible_choice_inner_complex() {
    let jkl = Extensible::Jkl(Basic::Ghi(1337));
    let (bits, buffer) = serialize_uper(&jkl);
    let jkl_deserialized = deserialize_uper(&buffer[..], bits);
    assert_eq!(jkl, jkl_deserialized);
}

#[test]
fn test_basic_variants_parsed() {
    let _abc = Basic::Abc(String::default());
    let _def = Basic::Def(String::default());
    let _ghi = Basic::Ghi(123_u64);

    match Basic::Abc(String::default()) {
        // this does not compile if there are additional unexpected variants
        Basic::Abc(_) | Basic::Def(_) | Basic::Ghi(_) => {}
    }
}

use asn1rs::prelude::basic::BasicRead;
use std::io::Read;

fn print(bin: &[u8], depth: u16) -> Vec<String> {
    let mut result = Vec::default();
    let reader = &mut &*bin;
    while !reader.is_empty() {
        let identifier = reader.read_identifier().unwrap();
        let len = reader.read_length().unwrap();

        let mut bin = core::iter::repeat(0u8)
            .take(len as usize)
            .collect::<Vec<_>>();

        reader.read_exact(&mut bin[..]).unwrap();

        result.push(format!(
            "{} - {identifier:?} {len} {bin:?}",
            core::iter::repeat(' ')
                .take(usize::from(depth) * 2)
                .collect::<String>()
        ));

        if identifier.value() as u8 & 0b0010_0000 != 0 {
            result.extend(print(&bin, depth + 1))
        }
    }
    result
}

fn print_compare(multiline: &str, bin: &[u8]) {
    let lines = print(bin, 0);
    let lines = lines.iter().map(String::as_str).collect::<Vec<_>>();
    lines.iter().for_each(|l| println!("{l}"));

    let lines_given = multiline.trim().lines().collect::<Vec<_>>();

    for i in 0..lines.len().min(lines_given.len()) {
        let expected = &lines_given[i];
        let got = &lines[i];
        assert_eq!(expected.trim(), got.trim());
    }
    assert_eq!(lines_given.len(), lines.len());
}

#[test]
fn print_simple_boolean() {
    print_compare(
        r#"
      - Universal(1) 1 [255]
     "#,
        &[0x01, 0x01, 0xFF],
    );
}

#[test]
fn print_letsencrypt_point_x_y() {
    print_compare(
        r#"
      - Universal(48) 6 [128, 1, 9, 129, 1, 9]
        - ContextSpecific(0) 1 [9]
        - ContextSpecific(1) 1 [9]
    "#,
        &[0x30, 0x06, 0x80, 0x01, 0x09, 0x81, 0x01, 0x09],
    );
}

#[test]
fn print_letsencrypt_sequence() {
    print_compare(
        r#"
      - Universal(48) 13 [6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 11, 5, 0]
        - Universal(6) 9 [42, 134, 72, 134, 247, 13, 1, 1, 11]
        - Universal(5) 0 []
    "#,
        &[
            0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x0b, 0x05,
            0x00,
        ],
    );
}

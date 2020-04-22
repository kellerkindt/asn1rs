use asn1rs_macros::asn;

#[asn(sequence)]
#[derive(Default)]
pub struct Potato {
    #[asn(Integer)]
    size: u64,
    #[asn(Utf8String)]
    string: String,
}

#[test]
fn test_compiles() {
    let _ = Potato {
        size: 123,
        string: String::from("where is the content"),
    };
}

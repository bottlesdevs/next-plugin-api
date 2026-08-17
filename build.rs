fn main() {
    let version = env!("CARGO_PKG_VERSION");
    let mut parts = version.split('.');
    let version = [
        parse(parts.next(), "major"),
        parse(parts.next(), "minor"),
        parse(parts.next(), "patch"),
    ];
    assert!(
        parts.next().is_none(),
        "plugin API version must be major.minor.patch"
    );

    let bytes = version
        .into_iter()
        .flat_map(u16::to_be_bytes)
        .collect::<Vec<_>>();
    std::fs::write(
        std::path::Path::new(&std::env::var_os("OUT_DIR").expect("OUT_DIR is set"))
            .join("api-version"),
        bytes,
    )
    .expect("write plugin API version marker");
}

fn parse(value: Option<&str>, name: &str) -> u16 {
    value
        .unwrap_or_else(|| panic!("plugin API version is missing its {name} number"))
        .parse()
        .unwrap_or_else(|_| panic!("plugin API {name} version must fit in u16"))
}

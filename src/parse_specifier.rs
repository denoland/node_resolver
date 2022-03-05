// Copyright 2022 the Deno authors. All rights reserved. MIT license.

pub fn parse_specifier(specifier: &str) -> Option<(String, String)> {
  let mut separator_index = specifier.find('/');
  let mut valid_package_name = true;
  // let mut is_scoped = false;
  if specifier.is_empty() {
    valid_package_name = false;
  } else if specifier.starts_with('@') {
    // is_scoped = true;
    if let Some(index) = separator_index {
      separator_index = specifier[index + 1..].find('/');
    } else {
      valid_package_name = false;
    }
  }

  let package_name = if let Some(index) = separator_index {
    specifier[0..index].to_string()
  } else {
    specifier.to_string()
  };

  // Package name cannot have leading . and cannot have percent-encoding or separators.
  for ch in package_name.chars() {
    if ch == '%' || ch == '\\' {
      valid_package_name = false;
      break;
    }
  }

  if !valid_package_name {
    return None;
  }

  let package_subpath = if let Some(index) = separator_index {
    format!(".{}", specifier.chars().skip(index).collect::<String>())
  } else {
    ".".to_string()
  };

  Some((package_name, package_subpath))
}

#[test]
fn test_parse_specifier() {
  let cases = vec![
    (
      "@parcel/css-darwin-arm64",
      Some(("@parcel/css-darwin-arm64", ".")),
    ),
    ("./path/to/directory", Some((".", "./path/to/directory"))),
    ("a-package/foo", Some(("a-package", "./foo"))),
    ("a-package/m.mjs", Some(("a-package", "./m.mjs"))),
    (
      "es-module-package/features/x",
      Some(("es-module-package", "./features/x")),
    ),
  ];
  for (input, expected) in cases {
    let r = parse_specifier(input);
    if let Some((package_name, subpath)) = expected {
      let actual = r.unwrap();
      assert_eq!(actual.0, package_name);
      assert!(actual.1.starts_with("./") || actual.1 == ".");
      assert_eq!(actual.1, subpath);
    } else {
      assert!(r.is_none());
    }
  }
}

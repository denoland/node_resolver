use serde::Deserialize;
use std::fs::read_to_string;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct PackageJson {
  name: String,
  r#type: Option<String>,
  exports: Option<String>,
}

pub fn node_resolve(
  specifier: &str,
  referrer: &Path,
) -> anyhow::Result<PathBuf> {
  if specifier.starts_with("./") || specifier.starts_with("../") {
    todo!();
  }
  if let Some(package_json_path) = find_package_json(specifier, referrer) {
    let s = read_to_string(&package_json_path)?;
    let package_json = parse_package_json(&s)?;
    println!("package_json {:?}", package_json);
    if let Some(exports) = package_json.exports {
      return Ok(package_json_path.parent().unwrap().join(exports));
    }
    todo!()
  } else {
    todo!()
  }
}

fn parse_package_json(s: &str) -> anyhow::Result<PackageJson> {
  let package_json = serde_json::from_str(s)?;
  Ok(package_json)
}

fn find_package_json(bare_specifier: &str, referrer: &Path) -> Option<PathBuf> {
  for a in referrer.ancestors() {
    let package_json = a
      .join("node_modules")
      .join(bare_specifier)
      .join("package.json");
    if package_json.exists() {
      return Some(package_json);
    }
  }
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn testdir(name: &str) -> PathBuf {
    let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.join("src/testdata/").join(name)
  }

  #[test]
  fn basic_esm() {
    let cwd = testdir("basic_esm");
    let referrer = cwd.join("main.js");
    let actual = node_resolve("foo", &referrer).unwrap();
    //assert!(matches!(actual, ResolveResponse::Esm(_)));
    assert_eq!(actual, cwd.join("node_modules/foo/index.js"));
  }

  #[test]
  fn package_json_parse() {
    let p = parse_package_json(
      r#"
        {
          "name": "bar",
          "type": "module",
          "dependencies": {
            "foo": "1.0.0"
          }
        }
     "#,
    )
    .unwrap();
    assert_eq!(p.name, "bar");
    assert_eq!(p.r#type.unwrap(), "module");
    assert!(p.exports.is_none());
  }
}

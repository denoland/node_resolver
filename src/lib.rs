mod package_json;
mod parse_specifier;

pub use package_json::PackageJson;
use parse_specifier::parse_specifier;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

pub fn resolve(
  specifier: &str,
  referrer: &Path,
  conditions: &[&str],
) -> anyhow::Result<PathBuf> {
  if specifier.starts_with('/') {
    todo!();
  }

  if specifier.starts_with("./") || specifier.starts_with("../") {
    if let Some(parent) = referrer.parent() {
      return file_extension_probe(parent.join(specifier));
    } else {
      todo!();
    }
  }

  // We've got a bare specifier or maybe bare_specifier/blah.js"

  let (package_name, package_subpath) = parse_specifier(specifier).unwrap();

  for ancestor in referrer.ancestors() {
    // println!("ancestor {:?}", ancestor);
    let module_dir = ancestor.join("node_modules").join(&package_name);
    let package_json_path = module_dir.join("package.json");
    if package_json_path.exists() {
      let package_json = PackageJson::load(package_json_path)?;

      if let Some(map) = package_json.exports_map {
        // println!("package_subpath {}", package_subpath);
        // println!("package_json exports_map {:#?}", map);

        if let Some((key, subpath)) = exports_resolve(&map, &package_subpath) {
          let value = map.get(&key).unwrap();
          let s = conditions_resolve(value, conditions);

          let t = resolve_package_target_string(&s, subpath);
          return Ok(module_dir.join(t));
        } else {
          todo!()
        }
      }

      // old school
      if package_subpath != "." {
        let d = module_dir.join(package_subpath);
        if let Ok(m) = d.metadata() {
          if m.is_dir() {
            return Ok(d.join("index.js"));
          }
        }
        return file_extension_probe(d);
      } else if let Some(main) = package_json.main {
        return Ok(module_dir.join(main));
      } else {
        return Ok(module_dir.join("index.js"));
      }
    }
  }

  Err(not_found())
}

// TODO needs some unit tests.
fn resolve_package_target_string(
  target: &str,
  subpath: Option<String>,
) -> String {
  if let Some(subpath) = subpath {
    // println!("target {}", target);
    // println!("subpath {:?}", subpath);
    target.replace('*', &subpath)
  } else {
    target.to_string()
  }
}

fn conditions_resolve(value: &Value, conditions: &[&str]) -> String {
  match value {
    Value::String(s) => s.to_string(),
    Value::Object(map) => {
      for condition in conditions {
        if let Some(x) = map.get(&condition.to_string()) {
          if let Value::String(s) = x {
            return s.to_string();
          } else {
            todo!()
          }
        }
      }
      todo!()
    }
    _ => todo!(),
  }
}

fn exports_resolve(
  map: &Map<String, Value>,
  subpath: &str,
) -> Option<(String, Option<String>)> {
  if map.contains_key(subpath) {
    return Some((subpath.to_string(), None));
  }

  // best match
  let mut best_match = None;
  for key in map.keys() {
    if let Some(pattern_index) = key.find('*') {
      let key_sub = &key[0..pattern_index];
      //println!("subpath {}", subpath);
      //println!("key_sub {}", key_sub);

      if subpath.starts_with(key_sub) {
        if subpath.ends_with('/') {
          todo!()
        }
        let pattern_trailer = &key[pattern_index + 1..];
        //println!("pattern_trailer {}", pattern_trailer);

        if subpath.len() > key.len()
          && subpath.ends_with(pattern_trailer)
          // && pattern_key_compare(best_match, key) == 1
          && key.rfind('*') == Some(pattern_index)
        {
          let rest = subpath
            [pattern_index..(subpath.len() - pattern_trailer.len())]
            .to_string();
          best_match = Some((key, rest));
        }
      }
    }
  }

  if let Some((key, subpath_)) = best_match {
    // println!("key {}", key);
    //println!("subpath_ {}", subpath_);
    return Some((key.to_string(), Some(subpath_)));
  }

  None
}

fn file_extension_probe(mut p: PathBuf) -> anyhow::Result<PathBuf> {
  if p.exists() {
    Ok(p)
  } else {
    p.set_extension("js");
    if p.exists() {
      Ok(p)
    } else {
      Err(not_found())
    }
  }
}

// TODO(bartlomieju): match error returned in Node
fn not_found() -> anyhow::Error {
  std::io::Error::new(
    std::io::ErrorKind::NotFound,
    "[ERR_MODULE_NOT_FOUND] Cannot find module",
  )
  .into()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn testdir(name: &str) -> PathBuf {
    let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.join("src/testdata").join(name)
  }

  fn check_node(main: &Path) {
    let status = std::process::Command::new("node")
      .args([main])
      .status()
      .unwrap();
    assert!(status.success());
  }

  #[test]
  fn cjs_no_main() {
    let d = testdir("cjs_no_main");
    let main_js = &d.join("main.js");
    check_node(main_js);
    let p = resolve("foo", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/index.js"));
  }

  #[test]
  fn cjs_main_basic() {
    let d = testdir("cjs_main");
    let main_js = &d.join("main.js");
    check_node(main_js);
    let p = resolve("foo", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/main.js"));
  }

  #[test]
  fn cjs_main_reach_inside() {
    let d = testdir("cjs_main");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("foo/bar.js", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/bar.js"));

    let p = resolve("foo/bar", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/bar.js"));

    let p = resolve("foo/dir/cat.js", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/cat.js"));

    let p = resolve("foo/dir/cat", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/cat.js"));

    let p = resolve("foo/dir", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/index.js"));

    let p = resolve("foo/dir/", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/index.js"));
  }

  #[test]
  fn cjs_main_not_found() {
    let d = testdir("cjs_main");
    let main_js = &d.join("main.js");
    check_node(main_js);
    let e = resolve("bar", main_js, &[]).unwrap_err();
    let ioerr = e.downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(ioerr.kind(), std::io::ErrorKind::NotFound);
  }

  #[test]
  fn cjs_main_sibling() {
    let d = testdir("cjs_main");
    let main_js = &d.join("main.js");
    check_node(main_js);
    let p = resolve("./sibling.js", main_js, &[]).unwrap();
    assert_eq!(p, d.join("sibling.js"));
    let p = resolve("./sibling", main_js, &[]).unwrap();
    assert_eq!(p, d.join("sibling.js"));
  }

  #[test]
  fn cjs_exports_string() {
    let d = testdir("cjs_exports_string");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("exports_string", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/exports_string/foo.js"));
  }

  #[test]
  fn cjs_exports_conditional() {
    let d = testdir("cjs_exports_conditional");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("exports", main_js, &["require"]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/main-require.cjs"));

    let p = resolve("exports", main_js, &["import"]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/main-module.js"));
  }

  #[test]
  fn cjs_exports_dot() {
    let d = testdir("cjs_exports_dot");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("exports", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/main-require.cjs"));
  }

  #[test]
  fn cjs_exports_multi() {
    let d = testdir("cjs_exports_multi");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("exports", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/main-require.cjs"));

    let p = resolve("exports/foo", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/foo.js"));

    let p = resolve("exports/bar/baz", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/exports/bar/baz.js"));
  }

  #[test]
  fn cjs_scoped() {
    let d = testdir("cjs_scoped");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let p = resolve("@ne-test-org/hello-world", main_js, &[]).unwrap();
    assert_eq!(p, d.join("node_modules/@ne-test-org/hello-world/index.js"));
  }
}

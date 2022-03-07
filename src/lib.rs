mod package_json;
mod parse_specifier;

use anyhow::bail;
use package_json::get_package_scope_config;
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
      return file_extension_probe(parent.join(specifier), referrer);
    } else {
      todo!();
    }
  }

  if specifier.starts_with('#') {
    return package_imports_resolve(specifier, referrer, conditions);
  }

  // We've got a bare specifier or maybe bare_specifier/blah.js"

  let (package_name, package_subpath) = parse_specifier(specifier).unwrap();

  for ancestor in referrer.ancestors() {
    let module_dir = ancestor.join("node_modules").join(&package_name);
    let package_json_path = module_dir.join("package.json");
    if package_json_path.exists() {
      let package_json = PackageJson::load(package_json_path)?;

      if let Some(map) = package_json.exports_map {
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
        return file_extension_probe(d, referrer);
      } else if let Some(main) = package_json.main {
        return Ok(module_dir.join(main));
      } else {
        return Ok(module_dir.join("index.js"));
      }
    }
  }

  Err(not_found(specifier, referrer))
}

// TODO needs some unit tests.
fn resolve_package_target_string(
  target: &str,
  subpath: Option<String>,
) -> String {
  if let Some(subpath) = subpath {
    target.replace('*', &subpath)
  } else {
    target.to_string()
  }
}

fn conditions_resolve(value: &Value, conditions: &[&str]) -> String {
  eprintln!("value {:#?} conds {:?}", value, conditions);
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
      if subpath.starts_with(key_sub) {
        if subpath.ends_with('/') {
          todo!()
        }
        let pattern_trailer = &key[pattern_index + 1..];

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
    return Some((key.to_string(), Some(subpath_)));
  }

  None
}

fn file_extension_probe(
  mut p: PathBuf,
  referrer: &Path,
) -> anyhow::Result<PathBuf> {
  if p.exists() {
    Ok(p)
  } else {
    p.set_extension("js");
    if p.exists() {
      Ok(p)
    } else {
      Err(not_found(&p.to_string_lossy(), referrer))
    }
  }
}

fn not_found(path: &str, referrer: &Path) -> anyhow::Error {
  let msg = format!(
    "[ERR_MODULE_NOT_FOUND] Cannot find \"{}\" imported from \"{}\"",
    path,
    referrer.to_string_lossy()
  );
  std::io::Error::new(std::io::ErrorKind::NotFound, msg).into()
}

fn package_imports_resolve(
  name: &str,
  referrer: &Path,
  conditions: &[&str],
) -> anyhow::Result<PathBuf> {
  if name == "#" || name.starts_with("#/") || name.ends_with('/') {
    let reason = "is not a valid internal imports specifier name";
    bail!("Invalid module specifier {} {}", name, reason);
    // return Err(errors::err_invalid_module_specifier(
    //   name,
    //   reason,
    //   Some(to_file_path_string(base)),
    // ));
  }

  let mut package_json_path = None;

  let package_config = get_package_scope_config(referrer)?;
  if package_config.exists {
    package_json_path = Some(package_config.path.clone());
    if let Some(imports) = &package_config.imports {
      if imports.contains_key(name) && !name.contains('*') {
        let maybe_resolved = resolve_package_target(
          package_json_path.clone().unwrap(),
          imports.get(name).unwrap().to_owned(),
          "".to_string(),
          name.to_string(),
          referrer,
          false,
          true,
          conditions,
        )?;
        if let Some(resolved) = maybe_resolved {
          return Ok(resolved);
        }
      } else {
        let mut best_match = "";
        let mut best_match_subpath = None;
        for key in imports.keys() {
          let pattern_index = key.find('*');
          if let Some(pattern_index) = pattern_index {
            let key_sub = &key[0..=pattern_index];
            if name.starts_with(key_sub) {
              let pattern_trailer = &key[pattern_index + 1..];
              if name.len() > key.len()
                && name.ends_with(&pattern_trailer)
                && pattern_key_compare(best_match, key) == 1
                && key.rfind('*') == Some(pattern_index)
              {
                best_match = key;
                best_match_subpath = Some(
                  name[pattern_index..=(name.len() - pattern_trailer.len())]
                    .to_string(),
                );
              }
            }
          }
        }

        if !best_match.is_empty() {
          let target = imports.get(best_match).unwrap().to_owned();
          let maybe_resolved = resolve_package_target(
            package_json_path.clone().unwrap(),
            target,
            best_match_subpath.unwrap(),
            best_match.to_string(),
            referrer,
            true,
            true,
            conditions,
          )?;
          if let Some(resolved) = maybe_resolved {
            return Ok(resolved);
          }
        }
      }
    }
  }

  bail!("Import not defined");
  // Err(throw_import_not_defined(name, package_json_url, base))
}

#[allow(clippy::too_many_arguments)]
fn resolve_package_target(
  package_json_path: &Path,
  target: Value,
  subpath: String,
  package_subpath: String,
  base: &Path,
  pattern: bool,
  internal: bool,
  conditions: &[&str],
) -> anyhow::Result<Option<PathBuf>> {
  if let Some(target) = target.as_str() {
    return Ok(Some(resolve_package_target_string(
      target.to_string(),
      subpath,
      package_subpath,
      package_json_path,
      base,
      pattern,
      internal,
      conditions,
    )?));
  } else if let Some(target_arr) = target.as_array() {
    if target_arr.is_empty() {
      return Ok(None);
    }

    let mut last_error = None;
    for target_item in target_arr {
      let resolved_result = resolve_package_target(
        package_json_path.clone(),
        target_item.to_owned(),
        subpath.clone(),
        package_subpath.clone(),
        base,
        pattern,
        internal,
        conditions,
      );

      if let Err(e) = resolved_result {
        let err_string = e.to_string();
        last_error = Some(e);
        if err_string.starts_with("[ERR_INVALID_PACKAGE_TARGET]") {
          continue;
        }
        return Err(last_error.unwrap());
      }
      let resolved = resolved_result.unwrap();
      if resolved.is_none() {
        last_error = None;
        continue;
      }
      return Ok(resolved);
    }
    if last_error.is_none() {
      return Ok(None);
    }
    return Err(last_error.unwrap());
  } else if let Some(target_obj) = target.as_object() {
    for key in target_obj.keys() {
      // TODO(bartlomieju): verify that keys are not numeric
      // return Err(errors::err_invalid_package_config(
      //   to_file_path_string(package_json_path),
      //   Some(base.as_str().to_string()),
      //   Some("\"exports\" cannot contain numeric property keys.".to_string()),
      // ));

      if key == "default" || conditions.contains(&key.as_str()) {
        let condition_target = target_obj.get(key).unwrap().to_owned();
        let resolved = resolve_package_target(
          package_json_path.clone(),
          condition_target,
          subpath.clone(),
          package_subpath.clone(),
          base,
          pattern,
          internal,
          conditions,
        )?;
        if resolved.is_none() {
          continue;
        }
        return Ok(resolved);
      }
    }
  } else if target.is_null() {
    return Ok(None);
  }

  bail!("Invalid package target");
  // Err(throw_invalid_package_target(
  //   package_subpath,
  //   target.to_string(),
  //   &package_json_path,
  //   internal,
  //   base,
  // ))
}

fn pattern_key_compare(a: &str, b: &str) -> i32 {
  let a_pattern_index = a.find('*');
  let b_pattern_index = b.find('*');

  let base_len_a = if let Some(index) = a_pattern_index {
    index + 1
  } else {
    a.len()
  };
  let base_len_b = if let Some(index) = b_pattern_index {
    index + 1
  } else {
    b.len()
  };

  if base_len_a > base_len_b {
    return -1;
  }

  if base_len_b > base_len_a {
    return 1;
  }

  if a_pattern_index.is_none() {
    return 1;
  }

  if b_pattern_index.is_none() {
    return -1;
  }

  if a.len() > b.len() {
    return -1;
  }

  if b.len() > a.len() {
    return 1;
  }

  0
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
    let msg = format!(
      "[ERR_MODULE_NOT_FOUND] Cannot find \"bar\" imported from \"{}\"",
      main_js.to_string_lossy()
    );
    assert_eq!(ioerr.to_string(), msg);
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

  #[test]
  fn conditional_exports() {
    // check that `exports` mapping works correctly
    let d = testdir("conditions");
    let main_js = &d.join("main.js");
    check_node(main_js);

    let actual =
      resolve("imports_exports", main_js, &["node", "import"]).unwrap();
    let expected = d.join("node_modules/imports_exports/import_export.js");
    assert_eq!(actual, expected);

    // check that `imports` mapping works correctly
    let d = testdir("conditions/node_modules/imports_exports");
    let main_js = &d.join("import_export.js");
    let actual = resolve("#dep", main_js, &[]).unwrap();
    let expected = d.join("import_polyfill.js");
    assert_eq!(actual, expected);
  }
}

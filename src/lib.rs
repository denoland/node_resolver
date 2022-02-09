use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug)]
struct PackageConfig {
  exists: bool,
  exports: Option<Value>,
  imports: Option<Map<String, Value>>,
  main: Option<String>,
  name: Option<String>,
  pjsonpath: PathBuf,
  typ: String,
}

fn get_package_config(
  path: PathBuf,
  /*
    specifier: &str,
    maybe_base: Option<&ModuleSpecifier>,
  */
) -> anyhow::Result<PackageConfig> {
  // TODO(bartlomieju):
  // if let Some(existing) = package_json_cache.get(path) {
  //   return Ok(existing.clone());
  // }

  let result = std::fs::read_to_string(&path);

  let source = result.unwrap_or_else(|_| "".to_string());
  if source.is_empty() {
    let package_config = PackageConfig {
      pjsonpath: path,
      exists: false,
      main: None,
      name: None,
      typ: "none".to_string(),
      exports: None,
      imports: None,
    };
    // TODO(bartlomieju):
    // package_json_cache.set(package_json_path, package_config.clone());
    return Ok(package_config);
  }

  let package_json: Value = serde_json::from_str(&source).map_err(|err| {
    /*
          let base_msg = maybe_base.map(|base| {
            format!("\"{}\" from {}", specifier, to_file_path(base).display())
          });
    errors::err_invalid_package_config(
      &path.display().to_string(),
      base_msg,
      Some(err.to_string()),
    )
    */
    anyhow::anyhow!("malformed json")
  })?;

  let imports_val = package_json.get("imports");
  let main_val = package_json.get("main");
  let name_val = package_json.get("name");
  let typ_val = package_json.get("type");
  let exports = package_json.get("exports").map(|e| e.to_owned());

  let imports = if let Some(imp) = imports_val {
    imp.as_object().map(|imp| imp.to_owned())
  } else {
    None
  };
  let main = if let Some(m) = main_val {
    m.as_str().map(|m| m.to_string())
  } else {
    None
  };
  let name = if let Some(n) = name_val {
    n.as_str().map(|n| n.to_string())
  } else {
    None
  };

  // Ignore unknown types for forwards compatibility
  let typ = if let Some(t) = typ_val {
    if let Some(t) = t.as_str() {
      if t != "module" && t != "commonjs" {
        "none".to_string()
      } else {
        t.to_string()
      }
    } else {
      "none".to_string()
    }
  } else {
    "none".to_string()
  };

  let package_config = PackageConfig {
    pjsonpath: path,
    exists: true,
    main,
    name,
    typ,
    exports,
    imports,
  };
  // TODO(bartlomieju):
  // package_json_cache.set(package_json_path, package_config.clone());
  Ok(package_config)
}

fn node_resolve(specifier: &str, referrer: &Path) -> anyhow::Result<PathBuf> {
  if specifier.starts_with("./") || specifier.starts_with("../") {
    todo!();
  }

  // We've got a bare specifier or maybe bare_specifier/blah.js"

  let (bare, maybe_rest) = if let Some((bare, rest)) = specifier.split_once("/")
  {
    (bare, Some(rest))
  } else {
    (specifier, None)
  };

  for ancestor in referrer.ancestors() {
    println!("ancestor {:?}", ancestor);
    let module_dir = ancestor.join("node_modules").join(bare);
    let package_json_path = module_dir.join("package.json");
    if package_json_path.exists() {
      //println!("path_json_path {:?}", package_json_path);
      let package_config = get_package_config(package_json_path)?;
      //println!("package_config {:?}", package_config);
      if let Some(rest) = maybe_rest {
        let d = module_dir.join(rest);
        if let Ok(m) = d.metadata() {
          if m.is_dir() {
            return Ok(d.join("index.js"));
          }
        }
        return Ok(d);
      } else {
        if let Some(main) = package_config.main {
          return Ok(module_dir.join(main));
        } else {
          return Ok(module_dir.join("index.js"));
        }
      }
    }
  }

  Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Not found").into())
}

#[cfg(test)]
mod tests {
  use super::*;

  fn testdir(name: &str) -> PathBuf {
    let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.join("src/testdata/").join(name)
  }

  #[test]
  fn cjs_no_main() {
    let d = testdir("cjs_no_main");
    let p = node_resolve("foo", &d.join("main.js")).unwrap();
    assert_eq!(p, d.join("node_modules/foo/index.js"));
  }

  #[test]
  fn cjs_main() {
    let d = testdir("cjs_main");
    let p = node_resolve("foo", &d.join("main.js")).unwrap();
    assert_eq!(p, d.join("node_modules/foo/main.js"));
  }

  #[test]
  fn cjs_main_reach_inside() {
    let d = testdir("cjs_main");
    let p = node_resolve("foo/bar.js", &d.join("main.js")).unwrap();
    assert_eq!(p, d.join("node_modules/foo/bar.js"));
    let p = node_resolve("foo/dir/cat.js", &d.join("main.js")).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/cat.js"));
    let p = node_resolve("foo/dir", &d.join("main.js")).unwrap();
    assert_eq!(p, d.join("node_modules/foo/dir/index.js"));
  }

  #[test]
  fn cjs_main_not_found() {
    let d = testdir("cjs_main");
    let e = node_resolve("bar", &d.join("main.js")).unwrap_err();
    let ioerr = e.downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(ioerr.kind(), std::io::ErrorKind::NotFound);
  }
}

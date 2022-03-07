use path_clean::PathClean;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct PackageJson {
  pub exists: bool,
  pub exports_map: Option<Map<String, Value>>,
  pub imports: Option<Map<String, Value>>,
  pub main: Option<String>,
  pub name: Option<String>,
  pub path: PathBuf,
  pub typ: String,
}

impl PackageJson {
  pub fn load(path: PathBuf) -> anyhow::Result<PackageJson> {
    let source = std::fs::read_to_string(&path)?;

    let package_json: Value = serde_json::from_str(&source)
      .map_err(|err| anyhow::anyhow!("malformed package.json {}", err))?;

    let imports_val = package_json.get("imports");
    let main_val = package_json.get("main");
    let name_val = package_json.get("name");
    let type_val = package_json.get("type");
    let exports_map = package_json.get("exports").map(|exports| {
      if is_conditional_exports_main_sugar(exports) {
        let mut map = Map::new();
        map.insert(".".to_string(), exports.to_owned());
        map
      } else {
        exports.as_object().unwrap().to_owned()
      }
    });

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
    let typ = if let Some(t) = type_val {
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

    let package_json = PackageJson {
      exists: true,
      path,
      main,
      name,
      typ,
      exports_map,
      imports,
    };
    Ok(package_json)
  }
}

fn is_conditional_exports_main_sugar(exports: &Value) -> bool {
  if exports.is_string() || exports.is_array() {
    return true;
  }

  if exports.is_null() || !exports.is_object() {
    return false;
  }

  let exports_obj = exports.as_object().unwrap();
  let mut is_conditional_sugar = false;
  let mut i = 0;
  for key in exports_obj.keys() {
    let cur_is_conditional_sugar = key.is_empty() || !key.starts_with('.');
    if i == 0 {
      is_conditional_sugar = cur_is_conditional_sugar;
      i += 1;
    } else if is_conditional_sugar != cur_is_conditional_sugar {
      panic!("\"exports\" cannot contains some keys starting with \'.\' and some not.
        The exports object must either be an object of package subpath keys
        or an object of main entry condition name keys only.")
    }
  }

  is_conditional_sugar
}

pub fn get_package_scope_config(
  referrer: &Path,
) -> anyhow::Result<PackageJson> {
  let mut package_json_path = referrer.join("./package.json").clean();

  loop {
    if package_json_path.ends_with("node_modules/package.json") {
      break;
    }

    let result = PackageJson::load(package_json_path.to_path_buf());
    // TODO(bartlomieju): ignores all errors, instead of checking for "No such file or directory"
    if let Ok(package_config) = result {
      return Ok(package_config);
    }

    let last_package_json_path = package_json_path.clone();
    package_json_path = package_json_path.join("../package.json").clean();
    // TODO(bartlomieju): I'm not sure this will work properly
    // Terminates at root where ../package.json equals ../../package.json
    // (can't just check "/package.json" for Windows support)
    if package_json_path == last_package_json_path {
      break;
    }
  }

  let package_config = PackageJson {
    exists: false,
    typ: "none".to_string(),
    exports_map: None,
    imports: None,
    main: None,
    name: None,
    path: PathBuf::new(),
  };

  // TODO(bartlomieju):
  // package_json_cache.set(package_json_path, package_config.clone());
  Ok(package_config)
}

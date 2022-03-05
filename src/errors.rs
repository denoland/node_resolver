// Copyright 2022 the Deno authors. All rights reserved. MIT license.

#![allow(dead_code)]

use anyhow::anyhow;
use anyhow::Error;
use std::path::Path;

pub(crate) fn err_invalid_module_specifier(
  request: &str,
  reason: &str,
  maybe_base: Option<&Path>,
) -> Error {
  let mut msg = format!(
    "[ERR_INVALID_MODULE_SPECIFIER] Invalid module \"{}\" {}",
    request, reason
  );

  if let Some(base) = maybe_base {
    msg = format!("{} imported from {}", msg, base.to_string_lossy());
  }

  // TODO(bartlomieju): should be TypeError
  anyhow!(msg)
}

pub(crate) fn err_invalid_package_config(
  path: &str,
  maybe_base: Option<&Path>,
  maybe_message: Option<String>,
) -> Error {
  let mut msg = format!(
    "[ERR_INVALID_PACKAGE_CONFIG] Invalid package config {}",
    path
  );

  if let Some(base) = maybe_base {
    msg = format!("{} while importing {}", msg, base.to_string_lossy());
  }

  if let Some(message) = maybe_message {
    msg = format!("{}. {}", msg, message);
  }

  anyhow!(msg)
}

pub(crate) fn err_module_not_found(
  path: &str,
  base: &Path,
  typ: &str,
) -> Error {
  anyhow!(format!(
    "[ERR_MODULE_NOT_FOUND] Cannot find {} \"{}\" imported from \"{}\"",
    typ,
    path,
    base.to_string_lossy()
  ))
}

pub(crate) fn err_unsupported_dir_import(path: &str, base: &Path) -> Error {
  let msg = format!(
    "[ERR_UNSUPPORTED_DIR_IMPORT] Directory import '{}' is not supported resolving ES modules imported from {}", 
    path, base.to_string_lossy()
  );
  anyhow!(msg)
}

pub(crate) fn err_invalid_package_target(
  pkg_path: String,
  key: String,
  target: String,
  is_import: bool,
  maybe_base: Option<&Path>,
) -> Error {
  let rel_error = !is_import && !target.is_empty() && !target.starts_with("./");
  let mut msg = "[ERR_INVALID_PACKAGE_TARGET]".to_string();

  if key == "." {
    assert!(!is_import);
    msg = format!("{} Invalid \"exports\" main target {} defined in the package config {}package.json", msg, target, pkg_path)
  } else {
    let ie = if is_import { "imports" } else { "exports" };
    msg = format!("{} Invalid \"{}\" target {} defined for '{}' in the package config {}package.json", msg, ie, target, key, pkg_path)
  };

  if let Some(base) = maybe_base {
    msg = format!("{} imported from {}", msg, base.to_string_lossy());
  };
  if rel_error {
    msg = format!("{}; target must start with \"./\"", msg);
  }

  anyhow!(msg)
}

pub(crate) fn err_package_path_not_exported(
  pkg_path: String,
  subpath: String,
  maybe_base: Option<&Path>,
) -> Error {
  let mut msg = "[ERR_PACKAGE_PATH_NOT_EXPORTED]".to_string();

  if subpath == "." {
    msg = format!(
      "{} No \"exports\" main defined in {}package.json",
      msg, pkg_path
    );
  } else {
    msg = format!("{} Package subpath \'{}\' is not defined by \"exports\" in {}package.json", msg, subpath, pkg_path);
  };

  if let Some(base) = maybe_base {
    msg = format!("{} imported from {}", msg, base.to_string_lossy());
  }

  anyhow!(msg)
}

pub(crate) fn err_package_import_not_defined(
  specifier: &str,
  package_path: Option<&Path>,
  base: &str,
) -> Error {
  let mut msg = format!(
    "[ERR_PACKAGE_IMPORT_NOT_DEFINED] Package import specifier \"{}\" is not defined in",
    specifier
  );

  if let Some(package_path) = package_path {
    msg = format!(
      "{} in package {}package.json",
      msg,
      package_path.to_string_lossy()
    );
  }

  msg = format!("{} imported from {}", msg, base);

  // TODO(bartlomieju): should be TypeError
  anyhow!(msg)
}

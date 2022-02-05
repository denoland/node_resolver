use std::path::Path;
use std::path::PathBuf;

pub fn resolve(
  specifier: &str,
  referrer: &str,
  cwd: &Path,
) -> Result<PathBuf, i32> {
  todo!()
}

#[cfg(test)]
mod tests {
  fn testdir(name: &str) -> PathBuf {
    let c = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    c.join("src/testdata/").join(name)
  }

  #[test]
  fn basic() {
    let cwd = testdir("basic");
    let main = cwd.join("main.js");
    let actual = resolve("foo", main.as_str(), &cwd).unwrap();
    let expected = cwd.join("node_modules/foo/index.js");
    //assert!(matches!(actual, ResolveResponse::Esm(_)));
    assert_eq!(actual.unwrap(), expected);

    /*
        let actual = node_resolve(
          "data:application/javascript,console.log(\"Hello%20Deno\");",
          main.as_str(),
          &cwd,
        )
        .unwrap();
        let expected =
          Url::parse("data:application/javascript,console.log(\"Hello%20Deno\");")
            .unwrap();
        assert!(matches!(actual, ResolveResponse::Specifier(_)));
        assert_eq!(actual.to_result().unwrap(), expected);
    */
  }
}

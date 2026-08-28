//! End-to-end checks for committed macro-expansion fixtures.

use std::{
    env,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::{Item, Meta, Token, punctuated::Punctuated};

#[test]
fn expansions_match() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = expansion_fixtures(&manifest_dir.join("tests/expand"));
    let project = ExpansionProject::new(&manifest_dir, &fixtures);

    for fixture in fixtures {
        let actual = project.expand(&fixture.input, &fixture.input_bin);

        if env::var_os("UPDATE_EXPANSIONS").is_some() {
            fs::write(&fixture.expected, prettyplease::unparse(&actual)).unwrap();
            let _ = fs::remove_file(project.actual_path(&fixture.input));
            eprintln!("{} - refreshed", fixture.expected.display());
            continue;
        }

        let expected = project.expand(&fixture.expected, &fixture.expected_bin);
        if !token_streams_equal(actual.to_token_stream(), expected.to_token_stream()) {
            let actual_path = project.actual_path(&fixture.input);
            fs::create_dir_all(actual_path.parent().unwrap()).unwrap();
            fs::write(&actual_path, prettyplease::unparse(&actual)).unwrap();
            panic!(
                "{} expanded differently\n\nexpected:\n{}\nactual:\n{}\nvisual diff:\n  nvim -d {} {}",
                fixture.input.display(),
                prettyplease::unparse(&expected),
                prettyplease::unparse(&actual),
                fixture.expected.display(),
                actual_path.display(),
            );
        }
        let _ = fs::remove_file(project.actual_path(&fixture.input));
        eprintln!("{} - ok", fixture.input.display());
    }
}

struct ExpansionFixture {
    input: PathBuf,
    expected: PathBuf,
    input_bin: String,
    expected_bin: String,
}

struct ExpansionProject {
    directory: PathBuf,
    manifest: PathBuf,
    target: PathBuf,
}

impl ExpansionProject {
    fn new(manifest_dir: &Path, fixtures: &[ExpansionFixture]) -> Self {
        let directory = manifest_dir
            .join("target/tests/token-expansions")
            .join(std::process::id().to_string());
        fs::create_dir_all(&directory).unwrap();

        let mut manifest = format!(
            "[package]\nname = \"token-expansion-tests\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n[dependencies]\nthese-macros-should-be-illegal = {{ path = {:?} }}\n\n[workspace]\n",
            manifest_dir,
        );
        for fixture in fixtures {
            writeln!(
                manifest,
                "\n[[bin]]\nname = {:?}\npath = {:?}",
                fixture.input_bin, fixture.input,
            )
            .unwrap();
            if fixture.expected.exists() {
                writeln!(
                    manifest,
                    "\n[[bin]]\nname = {:?}\npath = {:?}",
                    fixture.expected_bin, fixture.expected,
                )
                .unwrap();
            }
        }

        let manifest_path = directory.join("Cargo.toml");
        fs::write(&manifest_path, manifest).unwrap();

        Self {
            manifest: manifest_path,
            target: manifest_dir.join("target/tests/token-expansions/target"),
            directory,
        }
    }

    fn expand(&self, fixture: &Path, bin: &str) -> syn::File {
        let output = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
            .arg("expand")
            .arg("--manifest-path")
            .arg(&self.manifest)
            .arg("--bin")
            .arg(bin)
            .arg("--theme")
            .arg("none")
            .arg("--offline")
            .env("CARGO_TARGET_DIR", &self.target)
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "failed to expand {}:\n{}",
            fixture.display(),
            String::from_utf8_lossy(&output.stderr),
        );
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(
            !diagnostics
                .lines()
                .any(|line| line.trim_start().starts_with("error:")),
            "rustc recovered while expanding {}:\n{diagnostics}",
            fixture.display(),
        );

        let source = String::from_utf8(output.stdout).unwrap();
        let mut file = parse_file(fixture, &source);
        remove_injected_prelude(&mut file);
        file
    }

    fn actual_path(&self, fixture: &Path) -> PathBuf {
        let expanded = fixture.with_extension("expanded.rs");
        self.target
            .parent()
            .unwrap()
            .join("actual")
            .join(expanded.file_name().unwrap())
    }
}

impl Drop for ExpansionProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn expansion_fixtures(directory: &Path) -> Vec<ExpansionFixture> {
    let mut inputs: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "rs")
                && !path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".expanded.rs")
        })
        .collect();
    inputs.sort();
    inputs
        .into_iter()
        .enumerate()
        .map(|(index, input)| ExpansionFixture {
            expected: input.with_extension("expanded.rs"),
            input_bin: format!("expansion_{index}_input"),
            expected_bin: format!("expansion_{index}_expected"),
            input,
        })
        .collect()
}

fn parse_file(path: &Path, source: &str) -> syn::File {
    syn::parse_file(source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn remove_injected_prelude(file: &mut syn::File) {
    file.attrs.retain(|attribute| {
        let Meta::List(meta) = &attribute.meta else {
            return true;
        };
        if !meta.path.is_ident("feature") {
            return true;
        }

        meta.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_or(true, |features| {
                features.len() != 1
                    || !matches!(features.first(), Some(Meta::Path(path)) if path.is_ident("prelude_import"))
            })
    });

    file.items.retain(|item| match item {
        Item::Use(item) => !item
            .attrs
            .first()
            .is_some_and(|attribute| attribute.path().is_ident("prelude_import")),
        Item::ExternCrate(item) => item.ident != "std",
        _ => true,
    });
}

fn token_streams_equal(left: TokenStream, right: TokenStream) -> bool {
    let mut left = left.into_iter();
    let mut right = right.into_iter();

    loop {
        match (left.next(), right.next()) {
            (None, None) => return true,
            (Some(TokenTree::Group(left)), Some(TokenTree::Group(right))) => {
                if left.delimiter() != right.delimiter()
                    || !token_streams_equal(left.stream(), right.stream())
                {
                    return false;
                }
            }
            (Some(TokenTree::Ident(left)), Some(TokenTree::Ident(right))) => {
                if left != right {
                    return false;
                }
            }
            (Some(TokenTree::Punct(left)), Some(TokenTree::Punct(right))) => {
                if left.as_char() != right.as_char() || left.spacing() != right.spacing() {
                    return false;
                }
            }
            (Some(TokenTree::Literal(left)), Some(TokenTree::Literal(right))) => {
                if left.to_string() != right.to_string() {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

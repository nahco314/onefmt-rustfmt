use anyhow::Result;
use ignore::gitignore;
use rustfmt_nightly::{
    Config, Edition, FormatReportFormatterBuilder, Input, StyleEdition, Verbosity, Version,
};
use std::path::{Path, PathBuf};

struct NullOptions;

impl rustfmt_nightly::CliOptions for NullOptions {
    fn apply_to(self, _: &mut Config) {
        unreachable!();
    }
    fn config_path(&self) -> Option<&Path> {
        unreachable!();
    }

    fn edition(&self) -> Option<Edition> {
        unreachable!();
    }

    fn style_edition(&self) -> Option<StyleEdition> {
        unreachable!();
    }

    fn version(&self) -> Option<Version> {
        unreachable!();
    }
}

pub enum FormatResult {
    Success { formatted_content: String },
    Ignored,
    Error { error: String },
}

fn is_ignore(config: &Config, target_path: &PathBuf) -> Result<bool> {
    let ignore_list = &config.ignore();

    // Fast path: rustfmt `ignore` entries are commonly relative paths such as
    // `main.rs` or `src/foo.rs`. When these suffixes match, skip formatting
    // immediately to avoid platform-specific matching instability.
    for ignore_path in ignore_list {
        if target_path.ends_with(ignore_path) {
            return Ok(true);
        }
    }

    let mut ignore_builder = gitignore::GitignoreBuilder::new(ignore_list.rustfmt_toml_path());

    for ignore_path in ignore_list {
        let line = ignore_path.to_string_lossy();
        ignore_builder.add_line(None, &line)?;
    }

    let ignore_set = ignore_builder.build()?;

    Ok(ignore_set
        .matched_path_or_any_parents(&target_path, false)
        .is_ignore())
}

pub fn format(target_path: PathBuf, target_content: String) -> Result<FormatResult> {
    let (mut config, _) =
        rustfmt_nightly::load_config::<NullOptions>(Some(&target_path.parent().unwrap()), None)?;

    let mut out: Vec<u8> = Vec::with_capacity(target_content.len() * 2);
    config.set().emit_mode(rustfmt_nightly::EmitMode::Stdout);
    config.set().verbose(Verbosity::Quiet);
    config.set().skip_children(true);

    if is_ignore(&config, &target_path)? {
        return Ok(FormatResult::Ignored);
    }

    let mut session = rustfmt_nightly::Session::new(config, Some(&mut out));
    let res = session.format(Input::Text(target_content))?;

    let some_error = session.has_operational_errors()
        || session.has_parsing_errors()
        || session.has_formatting_errors()
        || session.has_check_errors()
        || session.has_diff()
        || session.has_unformatted_code_errors();

    if some_error {
        return Ok(FormatResult::Error {
            error: FormatReportFormatterBuilder::new(&res).build().to_string(),
        });
    }

    drop(session);

    let formatted = String::from_utf8(out)?;

    Ok(FormatResult::Success {
        formatted_content: formatted,
    })
}

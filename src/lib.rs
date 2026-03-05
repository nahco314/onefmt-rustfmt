#![feature(rustc_private)]

mod format;

use crate::format::{format, FormatResult};
use foro_plugin_utils::compat_util::get_target;
use foro_plugin_utils::data_json_utils::JsonGetter;
use foro_plugin_utils::foro_plugin_setup;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

pub fn main_with_json(input: Value) -> Value {
    let start = std::time::Instant::now();

    let target = match get_target(&input) {
        Ok(target) => target,
        Err(e) => {
            return json!({
                "plugin-panic": format!("failed to read target path: {e:#}"),
            });
        }
    };
    let target_content = match String::get_value(&input, ["target-content"]) {
        Ok(content) => content,
        Err(e) => {
            return json!({
                "plugin-panic": format!("failed to read target content: {e:#}"),
            });
        }
    };

    let result = match catch_unwind(AssertUnwindSafe(|| {
        format(PathBuf::from(target), target_content)
    })) {
        Ok(res) => match res {
        Ok(FormatResult::Success { formatted_content }) => {
            json!({
                "format-status": "success",
                "formatted-content": formatted_content,
            })
        }
        Ok(FormatResult::Ignored) => {
            json!({
                "format-status": "ignored",
            })
        }
        Ok(FormatResult::Error { error }) => {
            json!({
                "format-status": "error",
                "format-error": error,
            })
        }
        Err(e) => {
            json!({
                "plugin-panic": e.to_string(),
            })
        }
        },
        Err(_) => json!({
            "plugin-panic": "panic while running rustfmt plugin",
        }),
    };

    println!("time: {:?}", start.elapsed());

    result
}

foro_plugin_setup!(main_with_json);

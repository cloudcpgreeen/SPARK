//! SPARK 示例插件之二：`reverse` —— 输入倒序。
//! 与 `upper` 同一契约（`plugin-world`），宿主零改动即可加载。

mod bindings;

use bindings::exports::spark::runtime::plugin::{Guest, PluginError, PluginInfo};

struct Reverse;

impl Guest for Reverse {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "reverse".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "输入倒序".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        Ok(input.chars().rev().collect())
    }
}

bindings::export!(Reverse with_types_in bindings);

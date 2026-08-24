//! SPARK 示范插件：实现 `spark:runtime` 契约的 plugin 接口（世界 plugin-world）。
//!
//! 行为：
//! - `transform(input)` = 输入转大写；
//! - 输入以 `trap` 开头时 panic —— 宿主以 trap 形式捕获，验证沙箱隔离；
//! - 输入以 `err` 开头时返回 `err`（code `rejected`）—— 验证插件声明式失败。
//!
//! 构建：`cd spark-plugin && cargo component build`
//! 产物：`target/wasm32-unknown-unknown/release/spark_plugin.wasm`（组件，零 WASI import）

mod bindings;

use bindings::exports::spark::runtime::plugin::{Guest, PluginError, PluginInfo};

struct Upper;

impl Guest for Upper {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "upper".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "输入转大写".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        if input.starts_with("trap") {
            panic!("trap requested for input: {input}");
        }
        if input.starts_with("err") {
            return Err(PluginError {
                code: "rejected".into(),
                message: format!("拒绝处理: {input}"),
            });
        }
        Ok(input.to_uppercase())
    }
}

bindings::export!(Upper with_types_in bindings);

//! SPARK 示范插件：实现 `spark:runtime` 契约的 plugin 接口（世界 plugin-world）。
//!
//! 行为：
//! - `transform(input)` = 输入转大写；
//! - 输入以 `trap` 开头时 panic —— 宿主以 trap 形式捕获，验证沙箱隔离。
//!
//! 构建：`cd spark-plugin && cargo component build`
//! 产物：`target/wasm32-wasip2/release/spark_plugin.wasm`（组件，含 WIT 元数据）

mod bindings;

use bindings::exports::spark::runtime::plugin::Guest;

struct Upper;

impl Guest for Upper {
    fn name() -> String {
        "upper".into()
    }

    fn transform(input: String) -> String {
        if input.starts_with("trap") {
            panic!("trap requested for input: {input}");
        }
        input.to_uppercase()
    }
}

bindings::export!(Upper with_types_in bindings);

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

use bindings::exports::spark::runtime::plugin::{
    Guest, PluginError, PluginInfo, ToolParameter, ToolSchema,
};

struct Upper;

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

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

    fn schema() -> Vec<ToolSchema> {
        vec![ToolSchema {
            name: "upper".into(),
            description: "把输入文本转成大写".into(),
            parameters: vec![ToolParameter {
                name: "text".into(),
                parameter_type: "string".into(),
                description: "要转大写的文本".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        if tool != "upper" {
            return Err(err("tool", &format!("未知工具: {tool}")));
        }
        let v: serde_json::Value = serde_json::from_str(&args)
            .map_err(|_| err("args", "参数必须是 JSON 对象"))?;
        let text = v
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| err("args", "缺少 text 参数"))?;
        Self::transform(text.to_string())
    }
}

bindings::export!(Upper with_types_in bindings);

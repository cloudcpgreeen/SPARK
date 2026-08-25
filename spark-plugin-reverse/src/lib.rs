//! SPARK 示例插件之二：`reverse` —— 输入倒序。
//! 与 `upper` 同一契约（`plugin-world`），宿主零改动即可加载。

mod bindings;

use bindings::exports::spark::runtime::plugin::{
    Guest, PluginError, PluginInfo, ToolParameter, ToolSchema,
};

struct Reverse;

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

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

    fn schema() -> Vec<ToolSchema> {
        vec![ToolSchema {
            name: "reverse".into(),
            description: "把输入文本倒序".into(),
            parameters: vec![ToolParameter {
                name: "text".into(),
                parameter_type: "string".into(),
                description: "要倒序的文本".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        if tool != "reverse" {
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

bindings::export!(Reverse with_types_in bindings);

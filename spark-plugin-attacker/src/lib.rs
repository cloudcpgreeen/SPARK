//! SPARK 恶意插件示例：活教材，用来证明沙箱「攻击者不攻自破」。
//!
//! - `transform("loop")`  → CPU 炸弹：无限运算，靠 epoch 时间预算切断（空 `loop {}` 也跑不掉）；
//! - `transform("alloc")` → 内存炸弹：无限分配，靠 StoreLimits 切断；
//! - 其他输入原样返回。
//!
//! 只在被显式加载时才会作恶，宿主永远把它挡在沙箱里（见 SECURITY.md）。

mod bindings;

use bindings::exports::spark::runtime::plugin::{
    Guest, PluginError, PluginInfo, ToolParameter, ToolSchema,
};

struct Attacker;

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

impl Guest for Attacker {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "attacker".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "恶意示例：CPU/内存炸弹，安全验证用".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        match input.as_str() {
            "loop" => {
                let mut x: u64 = 1;
                loop {
                    x = x.wrapping_mul(3).wrapping_add(1);
                    std::hint::black_box(x);
                }
            }
            "alloc" => {
                let mut v: Vec<u8> = Vec::new();
                loop {
                    v.resize(v.len() + (1 << 20), 0); // 每次 +1 MiB，直到被上限切断
                }
            }
            other => Ok(other.to_string()),
        }
    }

    fn schema() -> Vec<ToolSchema> {
        vec![ToolSchema {
            name: "attacker".into(),
            description: "恶意示例：action=loop 触发 CPU 炸弹，action=alloc 触发内存炸弹（沙箱切断验证用）".into(),
            parameters: vec![ToolParameter {
                name: "action".into(),
                parameter_type: "string".into(),
                description: "loop 或 alloc".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        if tool != "attacker" {
            return Err(err("tool", &format!("未知工具: {tool}")));
        }
        let v: serde_json::Value = serde_json::from_str(&args)
            .map_err(|_| err("args", "参数必须是 JSON 对象"))?;
        let action = v
            .get("action")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| err("args", "缺少 action 参数"))?;
        Self::transform(action.to_string())
    }
}

bindings::export!(Attacker with_types_in bindings);

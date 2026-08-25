//! SPARK 真实业务插件之二：银行卡号 Luhn 校验 + 卡组织识别。
//! 与 idcard 组成「核验插件族」：同一 result 契约，宿主零改动。
//!
//! - Luhn：从右起每两位取一位翻倍（翻倍结果 ≥10 则个位十位相加），全部求和后 mod 10 == 0；
//! - 卡组织识别（按号段前缀）：4 → Visa，51–55 → Mastercard，34/37 → Amex，62 → UnionPay；
//! - 校验顺序：长度（13–19 位）→ 全数字 → 校验位。
//! 成功返回 `有效 · 卡组织`；失败返回带 code 的 err（length/format/checksum，非 trap）。

mod bindings;

use bindings::exports::spark::runtime::plugin::{
    Guest, PluginError, PluginInfo, ToolParameter, ToolSchema,
};

struct Luhn;

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

fn luhn_ok(digits: &[u8]) -> bool {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                (doubled / 10 + doubled % 10) as u32
            } else {
                *d as u32
            }
        })
        .sum();
    sum % 10 == 0
}

fn brand(id: &str) -> &'static str {
    if id.starts_with('4') {
        "Visa"
    } else if id.starts_with("34") || id.starts_with("37") {
        "Amex"
    } else if id.starts_with("51")
        || id.starts_with("52")
        || id.starts_with("53")
        || id.starts_with("54")
        || id.starts_with("55")
    {
        "Mastercard"
    } else if id.starts_with("62") {
        "UnionPay"
    } else {
        "未知"
    }
}

impl Guest for Luhn {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "luhn".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "银行卡号 Luhn 校验 + 卡组织识别".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        let id = input.trim();
        if !(13..=19).contains(&id.len()) {
            return Err(err("length", "长度错误：应为 13–19 位"));
        }
        let Some(digits) = id
            .chars()
            .map(|c| c.to_digit(10).map(|d| d as u8))
            .collect::<Option<Vec<_>>>()
        else {
            return Err(err("format", "格式错误：只能包含数字"));
        };
        if !luhn_ok(&digits) {
            return Err(err("checksum", "校验位错误"));
        }
        Ok(format!("有效 · {}", brand(id)))
    }

    fn schema() -> Vec<ToolSchema> {
        vec![ToolSchema {
            name: "luhn".into(),
            description: "银行卡号 Luhn 校验 + 卡组织识别（Visa/Mastercard/Amex/UnionPay）".into(),
            parameters: vec![ToolParameter {
                name: "card_number".into(),
                parameter_type: "string".into(),
                description: "13–19 位银行卡号".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        if tool != "luhn" {
            return Err(err("tool", &format!("未知工具: {tool}")));
        }
        let v: serde_json::Value = serde_json::from_str(&args)
            .map_err(|_| err("args", "参数必须是 JSON 对象"))?;
        let number = v
            .get("card_number")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| err("args", "缺少 card_number 参数"))?;
        Self::transform(number.to_string())
    }
}

bindings::export!(Luhn with_types_in bindings);

//! SPARK 真实业务插件之三：人民币金额转大写（财会大写，财政部《会计基础工作规范》）。
//! 与 idcard/luhn 组成「业务插件族」：同一 result 契约，宿主零改动。
//!
//! - 数字：零壹贰叁肆伍陆柒捌玖；单位：元角分；整数四位一组（拾佰仟 + 万/亿）。
//! - 中间有 0 写「零」（连续多个只写一个）；角位为 0 而分位非 0，元后写「零」；
//! - 到角写「整」、整数写「整」。失败返回带 code 的 err（format/precision/range，非 trap）。

mod bindings;

use bindings::exports::spark::runtime::plugin::{
    Guest, PluginError, PluginInfo, ToolParameter, ToolSchema,
};

struct Rmb;

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

const CN_DIGITS: [&str; 10] = ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"];
const SMALL_UNITS: [&str; 4] = ["", "拾", "佰", "仟"];

/// 0–9999 转中文，中间/尾部 0 正确插入「零」。
fn group_to_cn(g: u32) -> String {
    let mut s = String::new();
    let mut pending_zero = false;
    for pos in (0..4).rev() {
        let d = (g / 10u32.pow(pos)) % 10;
        if d == 0 {
            if !s.is_empty() {
                pending_zero = true;
            }
        } else {
            if pending_zero {
                s.push('零');
                pending_zero = false;
            }
            s.push_str(CN_DIGITS[d as usize]);
            s.push_str(SMALL_UNITS[pos as usize]);
        }
    }
    if s.is_empty() {
        "零".into()
    } else {
        s
    }
}

/// 整数部分转中文（万亿以下），组间正确插入「零」。
fn int_to_cn(n: u64) -> String {
    if n == 0 {
        return "零".into();
    }
    let yi = n / 100_000_000;
    let wan = (n / 10_000) % 10_000;
    let ge = n % 10_000;
    let mut s = String::new();
    if yi > 0 {
        s.push_str(&group_to_cn(yi as u32));
        s.push('亿');
    }
    if wan > 0 {
        if !s.is_empty() && wan < 1000 {
            s.push('零');
        }
        s.push_str(&group_to_cn(wan as u32));
        s.push('万');
    }
    if ge > 0 {
        if !s.is_empty() && ge < 1000 {
            s.push('零');
        }
        s.push_str(&group_to_cn(ge as u32));
    }
    s
}

/// 角分转中文（dec 为两位数字串）；角位为 0 而分位非 0 时元后写「零」，到角/整数写「整」。
fn dec_to_cn(dec: &str) -> String {
    let jiao = dec.as_bytes()[0] - b'0';
    let fen = dec.as_bytes()[1] - b'0';
    let mut s = String::new();
    if jiao == 0 && fen == 0 {
        s.push('整');
    } else {
        if jiao == 0 {
            s.push('零');
        } else {
            s.push_str(CN_DIGITS[jiao as usize]);
            s.push('角');
        }
        if fen > 0 {
            s.push_str(CN_DIGITS[fen as usize]);
            s.push('分');
        } else {
            s.push('整');
        }
    }
    s
}

/// 小写金额（如 `12345.67`）转中文大写；整数部分仅数字，小数最多两位（精确到分）。
fn rmb_to_upper(amount: &str) -> Result<String, PluginError> {
    let amount = amount.trim();
    let (int_part, dec_part) = amount
        .split_once('.')
        .map_or((amount, ""), |(i, d)| (i, d));
    if int_part.is_empty() || !int_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err("format", "金额格式错误：整数部分只能为数字"));
    }
    if !dec_part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(err("format", "金额格式错误：小数部分只能为数字"));
    }
    if dec_part.len() > 2 {
        return Err(err("precision", "金额最多精确到分"));
    }
    let int_val: u64 = int_part
        .parse()
        .map_err(|_| err("range", "金额超出支持范围（< 万亿）"))?;
    if int_val >= 1_000_000_000_000 {
        return Err(err("range", "金额超出支持范围（< 万亿）"));
    }
    let mut dec = dec_part.to_string();
    while dec.len() < 2 {
        dec.push('0');
    }
    Ok(format!("{}元{}", int_to_cn(int_val), dec_to_cn(&dec)))
}

impl Guest for Rmb {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "rmb".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "人民币金额转大写（财会大写）".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        rmb_to_upper(&input)
    }

    fn schema() -> Vec<ToolSchema> {
        vec![ToolSchema {
            name: "rmb".into(),
            description: "人民币金额小写转大写（财会大写，精确到分，万亿以下）".into(),
            parameters: vec![ToolParameter {
                name: "amount".into(),
                parameter_type: "string".into(),
                description: "金额，如 12345.67".into(),
            }],
        }]
    }

    fn invoke(tool: String, args: String) -> Result<String, PluginError> {
        if tool != "rmb" {
            return Err(err("tool", &format!("未知工具: {tool}")));
        }
        let v: serde_json::Value = serde_json::from_str(&args)
            .map_err(|_| err("args", "参数必须是 JSON 对象"))?;
        let amount = v
            .get("amount")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| err("args", "缺少 amount 参数"))?;
        Self::transform(amount.to_string())
    }
}

bindings::export!(Rmb with_types_in bindings);

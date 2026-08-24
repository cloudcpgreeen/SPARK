//! SPARK 真实业务插件：中国身份证号校验（GB 11643-1999，18 位）。
//! 首个真实算法插件：证明宿主零改动即可加载业务逻辑，且 `result` 的 err 承载有意义的
//! 结构化校验失败（code：length/format/date/checksum），不是 panic。
//!
//! 规则：
//! - 前 6 位地区码、8 位出生日期（YYYYMMDD）、3 位顺序码（第 17 位奇男偶女）、1 位校验码；
//! - 校验码 = 前 17 位按权重 [7,9,10,5,8,4,2,1,6,3,7,9,10,5,8,4,2] 加权和 mod 11，
//!   映射表 ['1','0','X','9','8','7','6','5','4','3','2']。
//!
//! 校验顺序：长度 → 前 17 位数字 → 出生日期合法 → 校验位。
//! 成功返回 `性别 · 出生日期 · 地区`；失败返回带 code 的 err（值，不是 trap）。

mod bindings;

use bindings::exports::spark::runtime::plugin::{Guest, PluginError, PluginInfo};

struct Idcard;

const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
const CHECKS: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];

fn err(code: &str, message: &str) -> PluginError {
    PluginError {
        code: code.into(),
        message: message.into(),
    }
}

fn check_birth(id: &str) -> Result<(), PluginError> {
    let (y, m, d): (u32, u32, u32) = (
        id[6..10].parse().unwrap(),
        id[10..12].parse().unwrap(),
        id[12..14].parse().unwrap(),
    );
    if !(1900..=2099).contains(&y) || !(1..=12).contains(&m) {
        return Err(err("date", "出生日期非法"));
    }
    let max_day = match m {
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=max_day).contains(&d) {
        return Err(err("date", "出生日期非法"));
    }
    Ok(())
}

fn check_sum(id: &str) -> bool {
    let sum: u32 = id[..17]
        .chars()
        .zip(WEIGHTS)
        .map(|(c, w)| c.to_digit(10).unwrap() * w)
        .sum();
    CHECKS[(sum % 11) as usize] == id.chars().nth(17).unwrap().to_ascii_uppercase()
}

impl Guest for Idcard {
    fn info() -> PluginInfo {
        PluginInfo {
            name: "idcard".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "中国身份证号校验（18 位）".into(),
        }
    }

    fn transform(input: String) -> Result<String, PluginError> {
        let id = input.trim();
        if id.len() != 18 {
            return Err(err("length", "长度错误：应为 18 位"));
        }
        if !id[..17].chars().all(|c| c.is_ascii_digit()) {
            return Err(err("format", "格式错误：前 17 位须为数字"));
        }
        check_birth(id)?;
        if !check_sum(id) {
            return Err(err("checksum", "校验位错误"));
        }
        let gender = if id.chars().nth(16).unwrap().to_digit(10).unwrap() % 2 == 1 {
            "男"
        } else {
            "女"
        };
        Ok(format!(
            "{gender} · {}-{}-{} · 地区 {}",
            &id[6..10],
            &id[10..12],
            &id[12..14],
            &id[0..6]
        ))
    }
}

bindings::export!(Idcard with_types_in bindings);

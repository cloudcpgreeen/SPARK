//! rmb 插件：人民币金额转大写端到端（含财会「零」/「整」规则与错误码）。
//! 组件缺失时跳过（先 `cd spark-plugin-rmb && cargo component build --release`）。

use spark_host::Host;

/// 解开 Ok；失败则 panic（PluginError 未实现 PartialEq，不能直接 assert_eq 整个 Result）。
fn expect_ok<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("期望成功，实际失败"),
    }
}

fn rmb_wasm() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../spark-plugin-rmb/target/wasm32-unknown-unknown/release/spark_plugin_rmb.wasm");
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[test]
fn amount_to_upper() {
    let Some(wasm) = rmb_wasm() else {
        eprintln!("skip: rmb 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let cases = [
        ("0", "零元整"),
        ("0.05", "零元零伍分"),
        ("0.50", "零元伍角整"),
        ("1", "壹元整"),
        ("1.00", "壹元整"),
        ("1.05", "壹元零伍分"),
        ("1.50", "壹元伍角整"),
        ("10.10", "壹拾元壹角整"),
        ("12.3", "壹拾贰元叁角整"),
        ("10000", "壹万元整"),
        ("10001.01", "壹万零壹元零壹分"),
        ("10010", "壹万零壹拾元整"),
        ("2003", "贰仟零叁元整"),
        ("100000001", "壹亿零壹元整"),
        ("100200300", "壹亿零贰拾万零叁佰元整"),
        ("12345.67", "壹万贰仟叁佰肆拾伍元陆角柒分"),
    ];
    for (input, expected) in cases {
        let (_, out) = host.run(&wasm, input).unwrap();
        assert_eq!(expect_ok(out), expected, "输入 `{input}`");
    }
}

#[test]
fn invalid_amount_returns_declared_errors() {
    let Some(wasm) = rmb_wasm() else {
        eprintln!("skip: rmb 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let cases = [
        ("12.345", "precision", "金额最多精确到分"),
        ("-5", "format", "金额格式错误：整数部分只能为数字"),
        ("abc", "format", "金额格式错误：整数部分只能为数字"),
        ("1000000000000", "range", "金额超出支持范围（< 万亿）"),
    ];
    for (input, code, message) in cases {
        let (_, out) = host.run(&wasm, input).unwrap();
        let Err(error) = out else {
            panic!("输入 `{input}` 应报 `{code}`");
        };
        assert_eq!(error.code, code, "输入 `{input}`");
        assert_eq!(error.message, message, "输入 `{input}`");
    }
}

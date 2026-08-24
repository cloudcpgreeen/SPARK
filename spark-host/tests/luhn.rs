//! luhn 插件：银行卡号 Luhn 校验 + 卡组织识别端到端。
//! 组件缺失时跳过（先 `cd spark-plugin-luhn && cargo component build --release`）。

use spark_host::Host;

fn luhn_wasm() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../spark-plugin-luhn/target/wasm32-unknown-unknown/release/spark_plugin_luhn.wasm",
    );
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[test]
fn valid_card_returns_brand() {
    let Some(wasm) = luhn_wasm() else {
        eprintln!("skip: luhn 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let (info, out) = host.run(&wasm, "4242424242424242").unwrap();
    assert_eq!(info.name, "luhn");
    assert_eq!(out, Ok("有效 · Visa".into()));
    let (_, out) = host.run(&wasm, "5105105105105100").unwrap();
    assert_eq!(out, Ok("有效 · Mastercard".into()));
    let (_, out) = host.run(&wasm, "378282246310005").unwrap();
    assert_eq!(out, Ok("有效 · Amex".into()));
}

#[test]
fn invalid_card_returns_declared_errors() {
    let Some(wasm) = luhn_wasm() else {
        eprintln!("skip: luhn 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let cases = [
        ("4242424242424243", "校验位错误"),
        ("4242-4242-4242-4242", "格式错误：只能包含数字"),
        ("123", "长度错误：应为 13–19 位"),
    ];
    for (input, want) in cases {
        let (_, out) = host.run(&wasm, input).unwrap();
        assert_eq!(out, Err(want.into()), "输入 `{input}` 应报 `{want}`");
    }
}

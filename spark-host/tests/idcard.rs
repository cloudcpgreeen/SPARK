//! idcard 插件：真实业务算法（身份证校验）端到端 —— 校验规则、err 语义、沙箱。
//! 组件缺失时跳过（先 `cd spark-plugin-idcard && cargo component build --release`）。

use spark_host::Host;

fn idcard_wasm() -> Option<String> {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../spark-plugin-idcard/target/wasm32-unknown-unknown/release/spark_plugin_idcard.wasm",
    );
    p.exists().then(|| p.to_string_lossy().into_owned())
}

#[test]
fn valid_id_extracts_info() {
    let Some(wasm) = idcard_wasm() else {
        eprintln!("skip: idcard 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let (info, out) = host.run(&wasm, "110101199001010015").unwrap();
    assert_eq!(info.name, "idcard");
    assert_eq!(out, Ok("男 · 1990-01-01 · 地区 110101".into()));
    let (_, out) = host.run(&wasm, "110101199001010023").unwrap();
    assert_eq!(out, Ok("女 · 1990-01-01 · 地区 110101".into()));
}

#[test]
fn invalid_id_returns_declared_errors() {
    let Some(wasm) = idcard_wasm() else {
        eprintln!("skip: idcard 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let cases = [
        ("123", "长度错误：应为 18 位"),
        ("11010A199001010015", "格式错误：前 17 位须为数字"),
        ("110101199013010015", "出生日期非法"),
        ("110101199001010010", "校验位错误"),
    ];
    for (input, want) in cases {
        let (_, out) = host.run(&wasm, input).unwrap();
        assert_eq!(out, Err(want.into()), "输入 `{input}` 应报 `{want}`");
    }
}

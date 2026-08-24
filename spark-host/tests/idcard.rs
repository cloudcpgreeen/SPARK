//! idcard 插件：真实业务算法（身份证校验）端到端 —— 校验规则、err 语义、沙箱。
//! 组件缺失时跳过（先 `cd spark-plugin-idcard && cargo component build --release`）。

use spark_host::Host;

/// 解开 Ok；失败则 panic（PluginError 未实现 PartialEq，不能直接 assert_eq 整个 Result）。
fn expect_ok<T, E>(r: Result<T, E>) -> T {
    match r {
        Ok(v) => v,
        Err(_) => panic!("期望成功，实际失败"),
    }
}

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
    assert_eq!(expect_ok(out), "男 · 1990-01-01 · 地区 110101");
    let (_, out) = host.run(&wasm, "110101199001010023").unwrap();
    assert_eq!(expect_ok(out), "女 · 1990-01-01 · 地区 110101");
}

#[test]
fn invalid_id_returns_declared_errors() {
    let Some(wasm) = idcard_wasm() else {
        eprintln!("skip: idcard 组件未构建");
        return;
    };
    let host = Host::new().unwrap();
    let cases = [
        ("123", "length", "长度错误：应为 18 位"),
        ("11010A199001010015", "format", "格式错误：前 17 位须为数字"),
        ("110101199013010015", "date", "出生日期非法"),
        ("110101199001010010", "checksum", "校验位错误"),
    ];
    for (input, code, message) in cases {
        let (_, out) = host.run(&wasm, input).unwrap();
        let Err(error) = out else {
            panic!("输入 `{input}` 应报 `{code}`");
        };
        assert_eq!(error.code, code);
        assert_eq!(error.message, message);
    }
}

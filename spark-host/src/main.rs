use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(wasm), Some(input)) = (args.next(), args.next()) else {
        eprintln!("usage: spark-host <plugin.wasm> <input>");
        return ExitCode::from(2);
    };
    match spark_host::run_plugin(&wasm, &input) {
        Ok((info, Ok(out))) => {
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            println!("output: {out}");
            ExitCode::SUCCESS
        }
        Ok((info, Err(msg))) => {
            // 插件声明式失败：结果是 err，不是崩溃。
            println!("plugin: {} {} — {}", info.name, info.version, info.description);
            eprintln!("plugin error: {msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            // 插件 trap = 可控错误：宿主不崩。
            eprintln!("plugin trap: {e}");
            ExitCode::SUCCESS
        }
    }
}

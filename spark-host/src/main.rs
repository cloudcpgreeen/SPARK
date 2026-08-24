use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(wasm), Some(input)) = (args.next(), args.next()) else {
        eprintln!("usage: spark-host <plugin.wasm> <input>");
        return ExitCode::from(2);
    };
    match spark_host::run_plugin(&wasm, &input) {
        Ok((name, out)) => {
            println!("plugin: {name}");
            println!("output: {out}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            // 插件 trap = 可控错误：宿主不崩。
            eprintln!("plugin trap: {e}");
            ExitCode::SUCCESS
        }
    }
}

//! SPARK 跨端域组件：headless Button 计数器（契约 `spark:ui@0.1.0`，世界 `domain-world`）。
//!
//! 同一个 `button.wasm` 跑在 Rust 后端（wasmtime）与 Web / RN 前端（jco）——
//! 组件本身不知道自己在哪一端。UI 是宿主的事，状态与行为在这里。
//! 零 import：不含任何 UI 概念，也不依赖宿主能力。
//!
//! 构建：`cargo component build --release`
//! 产物：`target/wasm32-unknown-unknown/release/button.wasm`

mod bindings;

use std::cell::Cell;

use bindings::exports::spark::ui::button::{Guest, GuestCounter};

struct Button;

impl Guest for Button {
    type Counter = Counter;
}

/// 域状态：只持有 count，不含 disabled / label / style 等任何 UI 概念。
struct Counter {
    count: Cell<u32>,
}

impl GuestCounter for Counter {
    fn new() -> Self {
        Self {
            count: Cell::new(0),
        }
    }

    fn click(&self) {
        self.count.set(self.count.get() + 1);
    }

    fn count(&self) -> u32 {
        self.count.get()
    }
}

bindings::export!(Button with_types_in bindings);

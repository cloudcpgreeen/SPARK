//! P2 Capability Contract Spike 的实验对象。
//!
//! 它 import `spark:capability/storage@0.1.0`，只发**裸 key**——不关心这个能力由谁实现、
//! 由什么介质承载。同一份 wasm 换 Host 实现，不重新编译。

mod bindings;

use std::cell::Cell;

use bindings::exports::spark::store::counter_store::{Guest, GuestCounter};
use bindings::spark::capability::storage;

const KEY: &str = "count";

struct StoreCounter;

impl Guest for StoreCounter {
    type Counter = Counter;
}

/// 状态住在组件实例里，**同时**写穿到 capability；构造时从 capability 读回来。
///
/// 靠「新实例读到什么」观察能力是否生效，而不是往契约里加 `persisted()` 之类的探针。
struct Counter {
    count: Cell<u32>,
}

impl GuestCounter for Counter {
    fn new() -> Self {
        // Ok(Some(v)) 用 v；Ok(None)（没有这个 key）与 Err（能力失败）都从 0 起。
        let count = match storage::get(KEY) {
            Ok(Some(v)) => v.parse().unwrap_or(0),
            _ => 0,
        };
        Self {
            count: Cell::new(count),
        }
    }

    fn click(&self) {
        let next = self.count.get() + 1;
        self.count.set(next);
        // ponytail: 写穿失败即静默——组件不处理 Err 分支。这个 Err 的可观测结果不在这里，
        // 而在「下一个新实例读到什么」：写失败 ⇒ 新实例读回旧值。
        let _ = storage::set(KEY, &next.to_string());
    }

    fn count(&self) -> u32 {
        self.count.get()
    }
}

bindings::export!(StoreCounter with_types_in bindings);

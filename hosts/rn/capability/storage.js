// RN Host 对 `spark:capability/storage@0.1.0` 的**注入点**。
//
// ⚠️ 这**不是**一个已验证的实现。RN runtime 在 P1 就停在「编译门通过、运行时未验证」，
// P2 没有改变这个状态。这里存在的意义只是标出「换 Host 时改的是哪一行」。
//
// 为什么这里是内存 Map 而不是 AsyncStorage：契约是**同步**的
// （`get: func(key: string) -> result<option<string>, store-error>`），
// 而 `AsyncStorage` 全是 Promise。把异步 API 塞进同步契约只有两条路：
//   1. 内存 Map + 异步落盘（写穿缓存）—— 代价是冷启动 hydration 有竞态，
//      需要 Host 在实例化组件之前 await 一次预热；
//   2. 把契约改成 async —— 会级联 wasmtime + wit-bindgen + jco + Metro，
//      而且 `future<T>` 在当前工具链实测不可用。
// 两条路都**不在 P2 回答**。这是实验结果该决定的事，不是先入为主的 API 设计。

// 命名空间是 Host 的实例策略，不是契约语义。
const NS = 'counter-store';

const mem = new Map();

export function get(key) {
  const v = mem.get(`${NS}:${key}`);
  // undefined = 没有这个 key（Ok(None)）；抛错才是能力失败（Err）。
  return v === undefined ? undefined : v;
}

export function set(key, value) {
  mem.set(`${NS}:${key}`, value);
}

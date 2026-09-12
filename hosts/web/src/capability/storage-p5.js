// P5 的 Web Host 对 `spark:capability/storage@0.1.0` 的实现。
//
// 与 P2 的 storage.js 是同一套 localStorage 实现，但有**两处刻意不同**：
//
//   1. ns 换成 `p5-counter-store`。P2 的 storage.js 写死 `counter-store`，
//      若 P5 复用它，同一页面里 P2 区块的点击会污染 P5 的数字。
//      P5 要证的是 Host 独立，不是共享 —— 这个 ns 是隔离的载体。
//
//   2. 多一个宿主级 deny 开关。G5 的反事实需要「同一个 Component artifact +
//      Host 侧的另一种 Capability 配置」。这是**宿主配置**，不是组件处理了错误分支。
//
// jco 的约定：成功就正常返回，失败要 throw 出 WIT 的 error 值
// （`throw { tag: 'denied', val: ... }`）。返回 `{ tag: 'err', ... }` 会被当成成功 —— 踩过。
//
// 无痕模式 / 存储被禁时 localStorage 本身就会抛异常，所以正常路径不需要假的开关。

const NS = 'p5-counter-store';

// 宿主级配置来自 URL：无状态、每次导航天然重置，因此**不占用任何存储**，
// 不会污染「state 由 Capability 决定」这个被观测对象。
const DENY_WRITES = new URLSearchParams(location.search).get('deny') === '1';

const k = (key) => `${NS}:${key}`;

function guard(fn) {
  try {
    return fn();
  } catch {
    throw { tag: 'denied', val: 'storage unavailable (private mode?)' };
  }
}

export function get(key) {
  return guard(() => {
    const v = localStorage.getItem(k(key));
    // undefined = 没有这个 key（Ok(None)）；抛错才是能力失败（Err）。
    return v === null ? undefined : v;
  });
}

export function set(key, value) {
  if (DENY_WRITES) {
    throw { tag: 'denied', val: 'read-only (P5 deny capability)' };
  }
  guard(() => localStorage.setItem(k(key), value));
}

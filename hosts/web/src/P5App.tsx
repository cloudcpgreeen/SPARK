import { useReducer, useState } from 'react';

import { counterStore } from './generated-p5/counter-store.js';

// P5：同一个冻结的 `counter-store.wasm`（sha256 未变、未重新编译），
// 由 Web Host 承载。与 App.tsx 的 P2 区块同构 —— React 只持有实例引用，
// count 住在 wasm 组件里，Capability 由本 Host 实现。
//
// 单独占一个页面而不是往 App.tsx 加 section：App.tsx 承载着 P1–P3 的验收证据，
// 不改它。
export default function P5App() {
  const [counter] = useState(() => new counterStore.Counter());
  const [, rerender] = useReducer((n: number) => n + 1, 0);

  const count = counter.count();

  return (
    <main>
      <h1>SPARK · P5</h1>

      <section>
        <h2>P5 · 同一个 artifact，另一个 Host</h2>
        <p>
          count: <strong data-testid="p5-count">{count}</strong>
        </p>
        <button
          data-testid="p5-click"
          onClick={() => {
            counter.click();
            rerender();
          }}
        >
          click
        </button>
        <p>
          <small>
            同一个冻结的 <code>counter-store.wasm</code>，由 Web Host 用{' '}
            <code>localStorage</code> 实现 <code>spark:capability/storage</code>。
          </small>
        </p>
      </section>
    </main>
  );
}

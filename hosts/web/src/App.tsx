import { useReducer, useState } from 'react';

import { button } from './generated/button.js';

export default function App() {
  // 唯一的 React 状态：组件实例的引用。count 不在 React 里。
  const [counter] = useState(() => new button.Counter());
  // 点击后只做一件事：让 React 重渲染。它不持有任何业务状态。
  const [, rerender] = useReducer((n: number) => n + 1, 0);

  // 每次渲染都向组件要值 —— 这个数字的唯一来源是 wasm 组件。
  const count = counter.count();

  return (
    <main>
      <h1>SPARK</h1>
      <p>
        count: <strong>{count}</strong>
      </p>
      <button
        onClick={() => {
          counter.click();
          rerender();
        }}
      >
        click
      </button>
      <p>
        <small>数字住在 button.wasm 里；React 只是把它画出来。</small>
      </p>
    </main>
  );
}

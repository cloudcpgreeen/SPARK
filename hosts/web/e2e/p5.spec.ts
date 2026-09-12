// P5 的 Web 垂证（G3 / G4 / G5）。
//
// 两个测试各自拿到一个**全新的 BrowserContext** —— 所以 localStorage 天然为空。
// 这不是约定，是断言：初始值不是 0 就直接失败，不许重试掩盖。
//
// 证据含义的边界（不许滑坡）：
//   - 测试一（normal）的 `reload → 3` **只证明** Web Host 的 localStorage Capability 在工作。
//   - 「Domain 与 Capability 分离」由**测试二的 deny 反事实**承担，不是靠 reload 本身。

import { expect, test } from '@playwright/test';

const COUNT_KEY = 'p5-counter-store:count';

const count = (page: import('@playwright/test').Page) => page.getByTestId('p5-count');
const click = (page: import('@playwright/test').Page) => page.getByTestId('p5-click');

test('G3/G4 · 同一个 artifact 在 Web Host 上跑出与 Rust Host 相同的 Domain 行为', async ({
  page,
}) => {
  await page.goto('/p5.html');

  // hermeticity：全新 context ⇒ 存储必须是干净的。不干净就是断言失败，不是 flake。
  await expect(count(page)).toHaveText('0');
  expect(await page.evaluate((k) => localStorage.getItem(k), COUNT_KEY)).toBeNull();

  for (let i = 0; i < 3; i++) await click(page).click();
  await expect(count(page)).toHaveText('3');

  // 值确实落在 Host 的 localStorage 里 —— 独立于组件的观测。
  expect(await page.evaluate((k) => localStorage.getItem(k), COUNT_KEY)).toBe('3');

  // 刷新：新实例从 capability 读回来。
  await page.reload();
  await expect(count(page)).toHaveText('3');
});

test('G5 · deny 反事实：Domain 行为不变，只有持久性改变', async ({ page }) => {
  // 宿主侧的第二种 Capability 配置。组件、WIT、artifact 全都没变。
  await page.goto('/p5.html?deny=1');

  await expect(count(page)).toHaveText('0');
  expect(await page.evaluate((k) => localStorage.getItem(k), COUNT_KEY)).toBeNull();

  for (let i = 0; i < 3; i++) await click(page).click();

  // **硬判据第一步，不能省**：坏 Capability 下 click ×3 仍然得到 3。
  // 若这一步失败，证明的是 capability failure propagation —— 不是 P5 要证的分离。
  await expect(count(page)).toHaveText('3');

  // 独立观测：写入从未落地。与 Rust 侧 `--deny` 的 `stored: 0 条` 同一个形状。
  expect(await page.evaluate((k) => localStorage.getItem(k), COUNT_KEY)).toBeNull();

  // 只在这里，持久性才改变：3 → 0。
  await page.reload();
  await expect(count(page)).toHaveText('0');
});

import { defineConfig } from '@playwright/test';

// P5 的 Web 验收：第一次让「Web Host 跑出 3」变成可断言的证据。
//
// 在此之前 P1–P3 的 Web 结果都是人眼看、手写进文档的散文 ——
// 没有脚本、没有 trace、没有 CI 产物。
//
// `reuseExistingServer` 让正在手工跑 `npm run dev` 的人不受干扰。
export default defineConfig({
  testDir: './e2e',
  use: { baseURL: 'http://localhost:5173' },
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173/p5.html',
    reuseExistingServer: true,
  },
});

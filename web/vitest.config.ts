import { defineConfig } from 'vitest/config'
import { fileURLToPath } from 'node:url'
export default defineConfig({
  resolve: { alias: { '@ant-design/pro-components': fileURLToPath(new URL('./node_modules/@ant-design/pro-components/es/index.js', import.meta.url)), '@umijs/max': fileURLToPath(new URL('./src/test/umi.ts', import.meta.url)) } },
  test: { server: { deps: { inline: [/antd/, /@ant-design/, /@rc-component/, /rc-/, /rc-util/] } }, testTimeout: 30000, environment: 'jsdom', include: ['src/**/*.test.{ts,tsx}'], setupFiles: ['./src/test/setup.ts'] },
})

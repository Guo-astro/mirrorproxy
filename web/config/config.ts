import { defineConfig } from '@umijs/max'

export default defineConfig({
  npmClient: 'npm',
  // React Router 7 exposes the former /server entry from its main module.
  alias: { 'react-router-dom/server': 'react-router-dom' },
  outputPath: 'dist',
  hash: true,
  esbuildMinifyIIFE: true,
  publicPath: '/',
  history: { type: 'browser' },
  chainWebpack(config) {
    // Umi's v6 file-prefix aliases swallow v7's /dom export. Keep exact roots.
    for (const name of ['react-router', 'react-router-dom']) {
      config.resolve.alias.delete(name)
      config.resolve.alias.set(`${name}$`, require.resolve(name))
    }
  },
  routes: [
    { path: '/', component: '@/pages/portal-route', layout: false },
    { path: '/login', component: '@/pages/account-route', layout: false },
    { path: '/account/*', component: '@/pages/account-route', layout: false },
    { path: '/admin/*', component: '@/pages/admin-route', layout: false },
    { path: '*', component: '@/pages/not-found', layout: false },
  ],
  title: 'MirrorProxy',
  favicons: ['/favicon.svg'],
  links: [
    { rel: 'apple-touch-icon', href: '/apple-touch-icon.png', sizes: '180x180' },
    { rel: 'manifest', href: '/site.webmanifest' },
  ],
  metas: [{ name: 'viewport', content: 'width=device-width, initial-scale=1' }, { name: 'theme-color', content: '#0b73fd' }],
  locale: { default: 'en-US', antd: true, baseNavigator: false },
  initialState: {},
  access: {},
  model: {},
  antd: {},
  mfsu: false,
  codeSplitting: { jsStrategy: 'granularChunks' },
  headScripts: [],
  proxy: process.env.MIRRORPROXY_DEV_API ? {
    '/api': { target: process.env.MIRRORPROXY_DEV_API, changeOrigin: false },
    '/admin/api': { target: process.env.MIRRORPROXY_DEV_API, changeOrigin: false },
    '/version': { target: process.env.MIRRORPROXY_DEV_API, changeOrigin: false },
  } : undefined,
})

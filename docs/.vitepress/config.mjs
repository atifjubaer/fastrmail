import { defineConfig } from 'vitepress';

export default defineConfig({
  title: 'FastrMail',
  description: 'High-performance, single-binary email server written in Rust with IMAP4rev2, JMAP, SMTP, Tantivy search, SpamGuard, and embedded Svelte 5 web clients.',
  head: [
    ['link', { rel: 'icon', href: '/favicon.ico' }],
    ['meta', { name: 'theme-color', content: '#3b82f6' }],
  ],
  themeConfig: {
    logo: '⚡',
    siteTitle: 'FastrMail',
    nav: [
      { text: 'Getting Started', link: '/guide/getting-started' },
      { text: 'Architecture', link: '/guide/architecture' },
      { text: 'SpamGuard', link: '/guide/spam-guard' },
      { text: 'JMAP & IMAP', link: '/guide/jmap' },
      { text: 'API Reference', link: '/guide/api-reference' },
      { text: 'GitHub', link: 'https://github.com/atifjubaer/fastrmail' },
    ],
    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Overview & Features', link: '/guide/getting-started' },
          { text: 'Docker & Dokploy Deployment', link: '/guide/docker' },
          { text: 'DNS Setup (SPF, DKIM, DMARC)', link: '/guide/dns' },
        ],
      },
      {
        text: 'Architecture & Protocols',
        items: [
          { text: 'System Architecture', link: '/guide/architecture' },
          { text: 'SMTP Inbound & Outbound Delivery', link: '/guide/smtp' },
          { text: 'IMAP4rev2 (RFC 9051)', link: '/guide/imap' },
          { text: 'JMAP Engine (RFC 8620/8621)', link: '/guide/jmap' },
          { text: 'Tantivy Full-Text Search', link: '/guide/search' },
          { text: 'SpamGuard (DNSBL & Greylisting)', link: '/guide/spam-guard' },
        ],
      },
      {
        text: 'Developer & Admin Reference',
        items: [
          { text: 'REST API & Webmail Endpoints', link: '/guide/api-reference' },
          { text: 'CLI Management Commands', link: '/guide/cli' },
        ],
      },
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/atifjubaer/fastrmail' },
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 FastrMail Contributors',
    },
    search: {
      provider: 'local',
    },
  },
});

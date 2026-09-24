<div align="center">

# FastrMail SaaS Landing Page

<p>
  <strong>Official marketing website & landing portal for FastrMail.</strong><br>
  Built with Next.js 15 (App Router), Tailwind CSS, Lucide Icons, and shadcn/ui.
</p>

<p>
  <a href="https://github.com/atifjubaer/fastrmail/releases"><img src="https://img.shields.io/badge/release-v1.0.0-blue.svg?style=flat-square" alt="Version"></a>
  <a href="https://nextjs.org/"><img src="https://img.shields.io/badge/Next.js-15.0-black.svg?style=flat-square&logo=next.js" alt="Next.js"></a>
  <a href="https://tailwindcss.com/"><img src="https://img.shields.io/badge/Tailwind-CSS-38B2AC.svg?style=flat-square&logo=tailwind-css" alt="Tailwind"></a>
  <a href="../LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow.svg?style=flat-square" alt="License"></a>
</p>

<p>
  <a href="#getting-started">Getting Started</a> •
  <a href="#features">Features</a> •
  <a href="#deployment-on-vercel">Vercel Deployment</a>
</p>

</div>

---

## ⚡ Features

- **Live GitHub Integration**: Dynamically fetches repository stars, tags, and latest release.
- **Enterprise Protocol Matrix**: Highlights FastrMail's SMTP, Submission (587), POP3 (110), IMAP4rev2, JMAP, and ManageSieve capabilities.
- **Interactive Comparison**: Side-by-side benchmark comparing FastrMail against Stalwart, Mailcow, and Postfix/Dovecot.
- **Responsive & Accessible**: Fully accessible UI with light/dark adaptive palettes powered by Tailwind CSS.

## 🚀 Getting Started

Run the development server locally:

```bash
npm install
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) with your browser to preview the site.

## 🛠️ Production Build

```bash
npm run build
npm run start
```

## 🌐 Deployment on Vercel

When importing the monorepo into Vercel:

1. **Root Directory**: In Vercel Project Settings → General, set **Root Directory** to `fastrmail-landing`.
2. **Framework Preset**: Automatically detected as `Next.js`.
3. **Build Command**: `next build` (default).
4. **Output Directory**: `.next` (default).

import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "FastrMail — The Open-Source Enterprise Mail Server",
  description:
    "High-performance, single-binary mail server written in Rust. SMTP, IMAP4rev2, JMAP, Tantivy search, SpamGuard, and embedded Svelte 5 webmail. Deploy in 60 seconds.",
  keywords: [
    "mail server",
    "email server",
    "SMTP",
    "IMAP",
    "JMAP",
    "Rust",
    "open source",
    "self-hosted",
    "Stalwart alternative",
    "Mailcow alternative",
  ],
  openGraph: {
    title: "FastrMail — The Open-Source Enterprise Mail Server",
    description:
      "A single-binary Rust mail server with SMTP, IMAP4rev2, JMAP, full-text search, and built-in spam defense. Free forever.",
    type: "website",
    url: "https://fastrmail.vercel.app",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body
        className={`${geistSans.variable} ${geistMono.variable} font-sans antialiased min-h-screen bg-background text-foreground`}
      >
        {children}
      </body>
    </html>
  );
}

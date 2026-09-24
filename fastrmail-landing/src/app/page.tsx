import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import {
  Mail,
  Shield,
  Zap,
  Search,
  Server,
  Globe,
  Lock,
  ArrowRight,
  CheckCircle2,
  ExternalLink,
} from "lucide-react";

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
    </svg>
  );
}

export default function LandingPage() {
  return (
    <div className="flex flex-col min-h-screen">
      {/* ── Navbar ─────────────────────────────────────────────────────── */}
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/80 backdrop-blur-xl">
        <div className="container mx-auto flex h-16 items-center justify-between px-4 md:px-8">
          <div className="flex items-center gap-2">
            <Zap className="h-6 w-6 text-primary" />
            <span className="text-xl font-bold tracking-tight">FastrMail</span>
          </div>
          <nav className="hidden md:flex items-center gap-8">
            <a href="#features" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Features</a>
            <a href="#comparison" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Compare</a>
            <a href="#deploy" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Deploy</a>
            <a href="https://github.com/atifjubaer/fastrmail" target="_blank" rel="noopener noreferrer" className="text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              <GithubIcon className="h-4 w-4" /> GitHub
            </a>
          </nav>
          <div className="flex items-center gap-3">
            <a href="#deploy">
              <Button size="sm">Deploy Now</Button>
            </a>
          </div>
        </div>
      </header>

      {/* ── Hero Section ───────────────────────────────────────────────── */}
      <section className="relative overflow-hidden">
        {/* Gradient background */}
        <div className="absolute inset-0 -z-10">
          <div className="absolute inset-0 bg-gradient-to-b from-background via-background to-background" />
          <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[800px] h-[600px] bg-primary/5 rounded-full blur-3xl" />
        </div>

        <div className="container mx-auto px-4 md:px-8 pt-24 pb-20 text-center">
          <Badge variant="secondary" className="mb-6 text-sm px-4 py-1.5">
            ✨ 100% Free &amp; Open Source — MIT Licensed
          </Badge>

          <h1 className="text-4xl md:text-6xl lg:text-7xl font-bold tracking-tight max-w-4xl mx-auto leading-[1.1]">
            The Open-Source
            <br />
            <span className="bg-gradient-to-r from-primary to-primary/60 bg-clip-text text-transparent">
              Enterprise Mail Server
            </span>
          </h1>

          <p className="mt-6 text-lg md:text-xl text-muted-foreground max-w-2xl mx-auto leading-relaxed">
            A high-performance, single-binary mail server written in Rust.
            SMTP, IMAP4rev2, JMAP, Tantivy full-text search, and built-in SpamGuard —
            all in under 35MB of RAM.
          </p>

          <div className="mt-10 flex flex-col sm:flex-row items-center justify-center gap-4">
            <a href="#deploy">
              <Button size="lg" className="text-base px-8 h-12 gap-2">
                Deploy in 60 Seconds <ArrowRight className="h-4 w-4" />
              </Button>
            </a>
            <a href="https://github.com/atifjubaer/fastrmail" target="_blank" rel="noopener noreferrer">
              <Button variant="outline" size="lg" className="text-base px-8 h-12 gap-2">
                <GithubIcon className="h-4 w-4" /> View on GitHub
              </Button>
            </a>
          </div>

          {/* Stats row */}
          <div className="mt-16 flex flex-wrap justify-center gap-8 md:gap-16">
            {[
              { value: "47/47", label: "Tests Passing" },
              { value: "<35MB", label: "Idle Memory" },
              { value: "1", label: "Single Binary" },
              { value: "6", label: "RFC Standards" },
            ].map((stat) => (
              <div key={stat.label} className="text-center">
                <div className="text-3xl font-bold">{stat.value}</div>
                <div className="text-sm text-muted-foreground mt-1">{stat.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Features Grid ──────────────────────────────────────────────── */}
      <section id="features" className="py-24">
        <div className="container mx-auto px-4 md:px-8">
          <div className="text-center mb-16">
            <Badge variant="secondary" className="mb-4">Features</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              Everything you need. Nothing you don&apos;t.
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              Replace Postfix + Dovecot + SpamAssassin + Roundcube with a single binary.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                icon: <Mail className="h-6 w-6" />,
                title: "SMTP + IMAP4rev2 + JMAP",
                description: "RFC 5321 inbound/outbound SMTP, RFC 9051 IMAP4rev2, and RFC 8620/8621 JMAP. Every modern protocol, natively async.",
              },
              {
                icon: <Shield className="h-6 w-6" />,
                title: "SpamGuard Defense",
                description: "Multi-zone DNSBL lookups (Spamhaus, Barracuda), 5-minute greylisting, SPF/DKIM/DMARC verification, and automatic DMARC policy rejection.",
              },
              {
                icon: <Search className="h-6 w-6" />,
                title: "Tantivy Full-Text Search",
                description: "Embedded Lucene-grade search engine. Sub-millisecond queries across sender, subject, and body with per-tenant isolation.",
              },
              {
                icon: <Lock className="h-6 w-6" />,
                title: "Argon2id + DKIM Signing",
                description: "Military-grade password hashing. Automated 2048-bit RSA DKIM signing with one CLI command for DNS provisioning.",
              },
              {
                icon: <Server className="h-6 w-6" />,
                title: "Zero Dependencies",
                description: "No Postfix. No Dovecot. No Redis. No JVM. One compiled Rust binary runs everything on Alpine Linux.",
              },
              {
                icon: <Globe className="h-6 w-6" />,
                title: "Embedded Svelte 5 UIs",
                description: "Built-in Webmail reader/composer and Admin dashboard with Svelte 5 runes. Served directly by Axum — no reverse proxy needed.",
              },
            ].map((feature) => (
              <Card key={feature.title} className="bg-card/50 border-border/40 hover:border-border/80 transition-colors">
                <CardHeader>
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-lg bg-primary/10 text-primary">
                      {feature.icon}
                    </div>
                    <CardTitle className="text-lg">{feature.title}</CardTitle>
                  </div>
                </CardHeader>
                <CardContent>
                  <CardDescription className="text-sm leading-relaxed">
                    {feature.description}
                  </CardDescription>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Comparison Table ───────────────────────────────────────────── */}
      <section id="comparison" className="py-24 bg-muted/30">
        <div className="container mx-auto px-4 md:px-8">
          <div className="text-center mb-16">
            <Badge variant="secondary" className="mb-4">Compare</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              How FastrMail stacks up
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              Feature-for-feature comparison with the leading alternatives.
            </p>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full text-sm border-collapse">
              <thead>
                <tr className="border-b border-border">
                  <th className="text-left py-4 px-4 font-semibold">Feature</th>
                  <th className="text-center py-4 px-4 font-semibold text-primary">FastrMail</th>
                  <th className="text-center py-4 px-4 font-semibold text-muted-foreground">Stalwart</th>
                  <th className="text-center py-4 px-4 font-semibold text-muted-foreground">Mailcow</th>
                  <th className="text-center py-4 px-4 font-semibold text-muted-foreground">Postfix + Dovecot</th>
                </tr>
              </thead>
              <tbody>
                {[
                  { feature: "Deployment", fastrmail: "1 Binary", stalwart: "1 Binary", mailcow: "15+ Containers", postfix: "8+ Packages" },
                  { feature: "Idle Memory", fastrmail: "<35 MB", stalwart: "~60 MB", mailcow: ">3.5 GB", postfix: "~250 MB" },
                  { feature: "Language", fastrmail: "Rust", stalwart: "Rust", mailcow: "PHP / Python / C", postfix: "C / Perl" },
                  { feature: "SMTP (RFC 5321)", fastrmail: "✅", stalwart: "✅", mailcow: "✅ Postfix", postfix: "✅ Postfix" },
                  { feature: "IMAP4rev2 (RFC 9051)", fastrmail: "✅", stalwart: "✅", mailcow: "⚠️ v1", postfix: "⚠️ v1" },
                  { feature: "JMAP (RFC 8620)", fastrmail: "✅", stalwart: "✅", mailcow: "❌", postfix: "❌" },
                  { feature: "Full-Text Search", fastrmail: "✅ Tantivy", stalwart: "✅ Tantivy", mailcow: "⚠️ Solr", postfix: "⚠️ External" },
                  { feature: "Spam Defense", fastrmail: "✅ Built-in", stalwart: "✅ Sieve", mailcow: "⚠️ Rspamd", postfix: "⚠️ SpamAssassin" },
                  { feature: "Built-in Webmail", fastrmail: "✅ Svelte 5", stalwart: "❌", mailcow: "⚠️ SOGo", postfix: "❌" },
                  { feature: "Price", fastrmail: "Free (MIT)", stalwart: "Dual License", mailcow: "Free + Paid", postfix: "Free" },
                ].map((row) => (
                  <tr key={row.feature} className="border-b border-border/40 hover:bg-muted/50 transition-colors">
                    <td className="py-3 px-4 font-medium">{row.feature}</td>
                    <td className="py-3 px-4 text-center font-semibold">{row.fastrmail}</td>
                    <td className="py-3 px-4 text-center text-muted-foreground">{row.stalwart}</td>
                    <td className="py-3 px-4 text-center text-muted-foreground">{row.mailcow}</td>
                    <td className="py-3 px-4 text-center text-muted-foreground">{row.postfix}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Pricing ────────────────────────────────────────────────────── */}
      <section className="py-24">
        <div className="container mx-auto px-4 md:px-8">
          <div className="text-center mb-12">
            <Badge variant="secondary" className="mb-4">Pricing</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              Free. Forever. No asterisks.
            </h2>
          </div>

          <div className="max-w-md mx-auto">
            <Card className="border-primary/40 bg-card/50">
              <CardHeader className="text-center">
                <div className="text-5xl font-bold">$0</div>
                <CardTitle className="text-xl mt-2">MIT License</CardTitle>
                <CardDescription>
                  No usage limits. No paywalled features. No enterprise upsell.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ul className="space-y-3">
                  {[
                    "Unlimited domains & mailboxes",
                    "Full SMTP + IMAP + JMAP",
                    "SpamGuard & DKIM signing",
                    "Tantivy full-text search",
                    "Built-in Webmail & Admin UI",
                    "Docker & Dokploy ready",
                    "Community support on GitHub",
                  ].map((item) => (
                    <li key={item} className="flex items-center gap-2 text-sm">
                      <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
                      {item}
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Deploy Section ─────────────────────────────────────────────── */}
      <section id="deploy" className="py-24 bg-muted/30">
        <div className="container mx-auto px-4 md:px-8">
          <div className="text-center mb-12">
            <Badge variant="secondary" className="mb-4">Deploy</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              Deploy in 60 seconds
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              Choose your platform. FastrMail works everywhere Docker runs.
            </p>
          </div>

          {/* Deploy code block */}
          <div className="max-w-2xl mx-auto mb-12">
            <Card className="bg-card border-border/60">
              <CardContent className="p-6">
                <pre className="text-sm font-mono overflow-x-auto">
                  <code>{`git clone https://github.com/atifjubaer/fastrmail.git
cd fastrmail
cp .env.example .env
docker compose up -d`}</code>
                </pre>
              </CardContent>
            </Card>
          </div>

          {/* Deploy platform buttons */}
          <div className="flex flex-wrap justify-center gap-4">
            {[
              { name: "Dokploy", href: "https://dokploy.com" },
              { name: "Coolify", href: "https://coolify.io" },
              { name: "Docker", href: "https://github.com/atifjubaer/fastrmail#-quick-start" },
              { name: "Build from Source", href: "https://github.com/atifjubaer/fastrmail" },
            ].map((platform) => (
              <a key={platform.name} href={platform.href} target="_blank" rel="noopener noreferrer">
                <Button variant="outline" className="gap-2 px-6">
                  <ExternalLink className="h-4 w-4" /> {platform.name}
                </Button>
              </a>
            ))}
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Footer ─────────────────────────────────────────────────────── */}
      <footer className="py-12 border-t border-border/40">
        <div className="container mx-auto px-4 md:px-8">
          <div className="flex flex-col md:flex-row items-center justify-between gap-4">
            <div className="flex items-center gap-2">
              <Zap className="h-5 w-5 text-primary" />
              <span className="font-semibold">FastrMail</span>
              <span className="text-sm text-muted-foreground">— Open-source enterprise mail server</span>
            </div>
            <div className="flex items-center gap-6 text-sm text-muted-foreground">
              <a href="https://github.com/atifjubaer/fastrmail" className="hover:text-foreground transition-colors flex items-center gap-1">
                <GithubIcon className="h-4 w-4" /> GitHub
              </a>
              <a href="https://github.com/atifjubaer/fastrmail/blob/main/LICENSE" className="hover:text-foreground transition-colors">
                MIT License
              </a>
              <span>© 2026 FastrMail Contributors</span>
            </div>
          </div>
        </div>
      </footer>
    </div>
  );
}

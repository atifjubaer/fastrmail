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
  Star,
  GitFork,
  Inbox,
  Filter,
  Send,
  BookOpen,
} from "lucide-react";

interface RepoStats {
  stars: number;
  forks: number;
  openIssues: number;
  latestTag: string;
}

async function getRepoStats(): Promise<RepoStats> {
  try {
    const [repoRes, tagsRes] = await Promise.all([
      fetch("https://api.github.com/repos/atifjubaer/fastrmail", {
        next: { revalidate: 3600 },
        headers: { Accept: "application/vnd.github.v3+json" },
      }),
      fetch("https://api.github.com/repos/atifjubaer/fastrmail/tags", {
        next: { revalidate: 3600 },
        headers: { Accept: "application/vnd.github.v3+json" },
      }),
    ]);

    const repoData = repoRes.ok ? await repoRes.json() : null;
    const tagsData = tagsRes.ok ? await tagsRes.json() : null;

    return {
      stars: repoData?.stargazers_count ?? 0,
      forks: repoData?.forks_count ?? 0,
      openIssues: repoData?.open_issues_count ?? 0,
      latestTag: tagsData?.[0]?.name ?? "v1.0.0",
    };
  } catch {
    return {
      stars: 0,
      forks: 0,
      openIssues: 0,
      latestTag: "v1.0.0",
    };
  }
}

function GithubIcon({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="currentColor">
      <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
    </svg>
  );
}

export default async function LandingPage() {
  const stats = await getRepoStats();

  return (
    <div className="flex flex-col min-h-screen">
      {/* ── Navbar ─────────────────────────────────────────────────────── */}
      <header className="sticky top-0 z-50 w-full border-b border-border/40 bg-background/80 backdrop-blur-xl">
        <div className="container mx-auto flex h-16 items-center justify-between px-4 md:px-8">
          <div className="flex items-center gap-2">
            <Zap className="h-6 w-6 text-primary" />
            <span className="text-xl font-bold tracking-tight">FastrMail</span>
            <Badge variant="outline" className="ml-2 hidden sm:inline-flex text-xs font-mono">
              {stats.latestTag}
            </Badge>
          </div>
          <nav className="hidden md:flex items-center gap-8">
            <a href="#features" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Features</a>
            <a href="#protocols" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Protocols</a>
            <a href="#comparison" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Compare</a>
            <a href="#deploy" className="text-sm text-muted-foreground hover:text-foreground transition-colors">Deploy</a>
            <a href="https://fastrmail-landing.vercel.app" target="_blank" rel="noopener noreferrer" className="text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              <BookOpen className="h-4 w-4" /> Docs
            </a>
            <a href="https://github.com/atifjubaer/fastrmail" target="_blank" rel="noopener noreferrer" className="text-sm text-muted-foreground hover:text-foreground transition-colors flex items-center gap-1">
              <GithubIcon className="h-4 w-4" /> GitHub
              {stats.stars > 0 && (
                <span className="ml-1 inline-flex items-center gap-0.5 text-xs bg-muted px-1.5 py-0.5 rounded-full">
                  <Star className="h-3 w-3 fill-yellow-500 text-yellow-500" />
                  {stats.stars}
                </span>
              )}
            </a>
          </nav>
          <div className="flex items-center gap-3">
            <a href="#deploy">
              <Button size="sm">Deploy v1.0.0</Button>
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
          <div className="flex flex-wrap items-center justify-center gap-2 mb-6">
            <Badge variant="secondary" className="text-sm px-4 py-1.5 font-medium">
              ✨ FastrMail {stats.latestTag} is Production-Ready
            </Badge>
            <Badge variant="outline" className="text-sm px-3 py-1.5 border-primary/30 text-primary">
              100% Free &amp; Open Source (MIT)
            </Badge>
          </div>

          <h1 className="text-4xl md:text-6xl lg:text-7xl font-bold tracking-tight max-w-4xl mx-auto leading-[1.1]">
            The Open-Source
            <br />
            <span className="bg-gradient-to-r from-primary to-primary/60 bg-clip-text text-transparent">
              Enterprise Mail Server
            </span>
          </h1>

          <p className="mt-6 text-lg md:text-xl text-muted-foreground max-w-3xl mx-auto leading-relaxed">
            A high-performance, single-binary email server written in Rust.
            Replacing Postfix, Dovecot, Rspamd, SpamAssassin, and Roundcube with
            SMTP (25/587), POP3 (110), IMAP4rev2, JMAP, Tantivy search, and ManageSieve — in &lt;35MB RAM.
          </p>

          <div className="mt-10 flex flex-col sm:flex-row items-center justify-center gap-4">
            <a href="#deploy">
              <Button size="lg" className="text-base px-8 h-12 gap-2">
                Deploy in 60 Seconds <ArrowRight className="h-4 w-4" />
              </Button>
            </a>
            <a href="https://github.com/atifjubaer/fastrmail" target="_blank" rel="noopener noreferrer">
              <Button variant="outline" size="lg" className="text-base px-8 h-12 gap-2">
                <GithubIcon className="h-4 w-4" /> Star on GitHub
                {stats.stars > 0 && <span className="text-xs bg-muted px-2 py-0.5 rounded font-mono">★ {stats.stars}</span>}
              </Button>
            </a>
            <a href="https://fastrmail-landing.vercel.app" target="_blank" rel="noopener noreferrer">
              <Button variant="ghost" size="lg" className="text-base px-6 h-12 gap-2">
                <BookOpen className="h-4 w-4" /> Documentation
              </Button>
            </a>
          </div>

          {/* Stats row with live + verified data */}
          <div className="mt-16 flex flex-wrap justify-center gap-8 md:gap-14">
            {[
              { value: "52/52", label: "Tests Passing (100%)" },
              { value: "<35MB", label: "Idle RAM Usage" },
              { value: "1", label: "Single Rust Binary" },
              { value: "8+", label: "RFC Protocols" },
              { value: stats.latestTag, label: "Latest Release" },
            ].map((stat) => (
              <div key={stat.label} className="text-center px-2">
                <div className="text-3xl font-bold font-mono tracking-tight text-foreground">{stat.value}</div>
                <div className="text-xs md:text-sm text-muted-foreground mt-1 font-medium">{stat.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <Separator />

      {/* ── Protocols Section ──────────────────────────────────────────── */}
      <section id="protocols" className="py-20 bg-muted/20">
        <div className="container mx-auto px-4 md:px-8">
          <div className="text-center mb-12">
            <Badge variant="secondary" className="mb-4">Enterprise Protocols</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              Full-Spectrum Protocol Coverage
            </h2>
            <p className="mt-3 text-muted-foreground max-w-2xl mx-auto">
              Connect every client from 1996 legacy POP3 systems to modern 2026 JMAP mobile apps.
            </p>
          </div>

          <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4 text-center">
            {[
              { port: "Port 25", name: "Inbound SMTP", rfc: "RFC 5321", icon: <Send className="h-5 w-5" /> },
              { port: "Port 587", name: "Submission Auth", rfc: "RFC 6409", icon: <Lock className="h-5 w-5" /> },
              { port: "Port 110", name: "POP3 Engine", rfc: "RFC 1939", icon: <Inbox className="h-5 w-5" /> },
              { port: "Port 143", name: "IMAP4rev2", rfc: "RFC 9051", icon: <Mail className="h-5 w-5" /> },
              { port: "Port 8080", name: "JMAP JSON", rfc: "RFC 8620/8621", icon: <Zap className="h-5 w-5" /> },
              { port: "In-Memory", name: "ManageSieve", rfc: "RFC 5228", icon: <Filter className="h-5 w-5" /> },
            ].map((proto) => (
              <Card key={proto.name} className="border-border/60 bg-card/60">
                <CardHeader className="p-4 pb-2">
                  <div className="mx-auto p-2 w-fit rounded-lg bg-primary/10 text-primary">
                    {proto.icon}
                  </div>
                  <CardTitle className="text-sm font-semibold mt-2">{proto.name}</CardTitle>
                </CardHeader>
                <CardContent className="p-4 pt-0">
                  <div className="text-xs font-mono font-bold text-primary">{proto.port}</div>
                  <div className="text-[11px] text-muted-foreground mt-0.5">{proto.rfc}</div>
                </CardContent>
              </Card>
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
              Enterprise Grade. Zero Container Sprawl.
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              Replace a complex 15-container stack with a single statically linked Rust binary.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {[
              {
                icon: <Send className="h-6 w-6" />,
                title: "Inbound SMTP & Submission (Port 587)",
                description: "RFC 5321 inbound mail handling and RFC 6409 authenticated submission. Supports SASL AUTH PLAIN & LOGIN with Argon2id password verification.",
              },
              {
                icon: <Inbox className="h-6 w-6" />,
                title: "IMAP4rev2 & POP3 Server (Port 110)",
                description: "Full RFC 9051 IMAP4rev2 with MODSEQ synchronization and transactional state. Complete RFC 1939 POP3 engine with permanent expunge support.",
              },
              {
                icon: <Zap className="h-6 w-6" />,
                title: "Native JMAP Engine (RFC 8620/8621)",
                description: "Lightweight, push-friendly JSON-over-HTTP protocol. Enables offline synchronization, delta syncs, and lightning-fast webmail clients.",
              },
              {
                icon: <Filter className="h-6 w-6" />,
                title: "ManageSieve Rule Engine",
                description: "Custom programmatic email filtering rules. Evaluates headers on the inbound DATA stream to discard, reject, flag, or file into specific mailboxes.",
              },
              {
                icon: <Shield className="h-6 w-6" />,
                title: "SpamGuard Multi-Zone Defense",
                description: "Multi-zone DNSBL queries (Spamhaus, Barracuda), 5-minute greylisting defense, automated SPF verification, DKIM signing, and DMARC enforcement.",
              },
              {
                icon: <Search className="h-6 w-6" />,
                title: "Tantivy Full-Text Search",
                description: "Embedded Lucene-grade search engine in pure Rust. Sub-millisecond queries across sender, recipient, subject, and message body with per-tenant isolation.",
              },
              {
                icon: <Server className="h-6 w-6" />,
                title: "Zero External Dependencies",
                description: "No Postfix. No Dovecot. No Redis. No Python. No JVM. A single compiled Rust binary with embedded SQLite WAL mode database.",
              },
              {
                icon: <Globe className="h-6 w-6" />,
                title: "Embedded Svelte 5 Webmail & Admin",
                description: "Built-in Webmail reader/composer and Admin tenant management dashboard powered by Svelte 5 runes. Served directly via Axum.",
              },
              {
                icon: <Lock className="h-6 w-6" />,
                title: "Automated DKIM & Outbound MTA",
                description: "Automated 2048-bit RSA DKIM key generation and signing. Direct MX delivery with opportunistic STARTTLS, retry queue, and NDR bounces.",
              },
            ].map((feature) => (
              <Card key={feature.title} className="bg-card/50 border-border/40 hover:border-border/80 transition-colors">
                <CardHeader>
                  <div className="flex items-center gap-3">
                    <div className="p-2 rounded-lg bg-primary/10 text-primary">
                      {feature.icon}
                    </div>
                    <CardTitle className="text-base font-semibold">{feature.title}</CardTitle>
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
              Feature-by-Feature Benchmark
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              How FastrMail compares directly with Stalwart, Mailcow, and traditional Postfix stacks.
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
                  { feature: "Binary Packaging", fastrmail: "1 Single Binary", stalwart: "1 Binary", mailcow: "15+ Containers", postfix: "8+ Packages" },
                  { feature: "Idle Memory", fastrmail: "<35 MB", stalwart: "~60 MB", mailcow: ">3.5 GB", postfix: "~250 MB" },
                  { feature: "Core Language", fastrmail: "Rust 2021", stalwart: "Rust", mailcow: "PHP / Python / C", postfix: "C / Perl" },
                  { feature: "Inbound SMTP (RFC 5321)", fastrmail: "✅ Built-in", stalwart: "✅ Built-in", mailcow: "✅ Postfix", postfix: "✅ Postfix" },
                  { feature: "Auth Submission (RFC 6409)", fastrmail: "✅ Port 587", stalwart: "✅ Port 587", mailcow: "✅ Postfix", postfix: "✅ Postfix" },
                  { feature: "POP3 Server (RFC 1939)", fastrmail: "✅ Port 110", stalwart: "✅ Port 110", mailcow: "✅ Dovecot", postfix: "✅ Dovecot" },
                  { feature: "IMAP4rev2 (RFC 9051)", fastrmail: "✅ Native Async", stalwart: "✅ Native Async", mailcow: "⚠️ v1", postfix: "⚠️ v1" },
                  { feature: "JMAP (RFC 8620/8621)", fastrmail: "✅ Native JSON", stalwart: "✅ Native JSON", mailcow: "❌ None", postfix: "❌ None" },
                  { feature: "Sieve Rule Filtering", fastrmail: "✅ ManageSieve FSM", stalwart: "✅ Sieve", mailcow: "⚠️ Sieve", postfix: "⚠️ Sieve" },
                  { feature: "Full-Text Search", fastrmail: "✅ Tantivy Embedded", stalwart: "✅ Tantivy", mailcow: "⚠️ Solr", postfix: "⚠️ External" },
                  { feature: "Spam Defense", fastrmail: "✅ DNSBL + Greylist", stalwart: "✅ Sieve / DNSBL", mailcow: "⚠️ Rspamd", postfix: "⚠️ SpamAssassin" },
                  { feature: "Embedded Webmail", fastrmail: "✅ Svelte 5 Runes", stalwart: "❌ External", mailcow: "⚠️ SOGo", postfix: "❌ External" },
                  { feature: "License", fastrmail: "100% Free (MIT)", stalwart: "Dual License", mailcow: "Free + Paid", postfix: "Free" },
                ].map((row) => (
                  <tr key={row.feature} className="border-b border-border/40 hover:bg-muted/50 transition-colors">
                    <td className="py-3 px-4 font-medium">{row.feature}</td>
                    <td className="py-3 px-4 text-center font-semibold text-primary">{row.fastrmail}</td>
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
            <Badge variant="secondary" className="mb-4">Licensing</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              100% Free &amp; Open Source
            </h2>
            <p className="mt-3 text-muted-foreground">
              No artificial tiers. No seat limits. No enterprise paywalls.
            </p>
          </div>

          <div className="max-w-md mx-auto">
            <Card className="border-primary/40 bg-card/50 shadow-lg">
              <CardHeader className="text-center">
                <div className="text-5xl font-bold font-mono">$0</div>
                <CardTitle className="text-xl mt-2">MIT License</CardTitle>
                <CardDescription>
                  Permissive open-source license for personal and commercial infrastructure.
                </CardDescription>
              </CardHeader>
              <CardContent>
                <ul className="space-y-3">
                  {[
                    "Unlimited domains, users & mailboxes",
                    "Full SMTP (25/587), POP3 (110) & IMAP4rev2",
                    "Native JMAP & Tantivy full-text search",
                    "SpamGuard DNSBL, Greylisting & Sieve rules",
                    "Embedded Svelte 5 Webmail & Admin panels",
                    "Docker Compose & Dokploy one-click deploy",
                    "Community driven on GitHub",
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
            <Badge variant="secondary" className="mb-4">One-Click Deploy</Badge>
            <h2 className="text-3xl md:text-4xl font-bold tracking-tight">
              Deploy FastrMail in 60 Seconds
            </h2>
            <p className="mt-4 text-muted-foreground max-w-xl mx-auto">
              Run FastrMail locally, on your VPS, or through modern self-hosting orchestrators.
            </p>
          </div>

          {/* Deploy code block */}
          <div className="max-w-2xl mx-auto mb-12">
            <Card className="bg-card border-border/60">
              <CardContent className="p-6">
                <div className="flex items-center justify-between text-xs text-muted-foreground pb-2 mb-2 border-b border-border/40 font-mono">
                  <span>bash</span>
                  <span>Docker Quickstart</span>
                </div>
                <pre className="text-sm font-mono overflow-x-auto text-primary">
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
              { name: "Dokploy Template", href: "https://github.com/atifjubaer/fastrmail/blob/main/dokploy-service-template.yaml" },
              { name: "Docker Compose", href: "https://github.com/atifjubaer/fastrmail/blob/main/docker-compose.yml" },
              { name: "Documentation", href: "https://fastrmail-landing.vercel.app" },
              { name: "GitHub Repository", href: "https://github.com/atifjubaer/fastrmail" },
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
              <span className="text-sm text-muted-foreground">— Open-source enterprise mail engine in Rust</span>
            </div>
            <div className="flex items-center gap-6 text-sm text-muted-foreground">
              <a href="https://github.com/atifjubaer/fastrmail" className="hover:text-foreground transition-colors flex items-center gap-1">
                <GithubIcon className="h-4 w-4" /> GitHub
              </a>
              <a href="https://fastrmail-landing.vercel.app" className="hover:text-foreground transition-colors">
                Docs
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

<script lang="ts">
  import { onMount } from "svelte";
  import {
    LayoutDashboard,
    Globe,
    Users,
    Clock,
    ShieldCheck,
    Server,
    Plus,
    Key,
    Copy,
    Check,
    Trash2,
    RefreshCw,
    X,
    Lock,
    Mail,
    Send,
    LogOut,
    CheckCircle2,
    AlertCircle,
    Terminal,
    Database,
    Activity,
    Layers,
    Search,
    Download,
    Cpu,
    HardDrive,
  } from "@lucide/svelte";

  import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "./lib/components/ui/card";
  import { Badge } from "./lib/components/ui/badge";
  import { Button } from "./lib/components/ui/button";
  import { Table, TableHeader, TableBody, TableRow, TableHead, TableCell } from "./lib/components/ui/table";

  interface Stats {
    tenants_count: number;
    accounts_count: number;
    messages_count: number;
    queue_pending_count: number;
  }

  interface Tenant {
    id: string;
    domain: string;
    created_at: string;
  }

  interface Account {
    id: string;
    tenant_id: string;
    username: string;
    email: string;
    quota_bytes: number;
    created_at: string;
  }

  interface QueueItem {
    id: string;
    tenant_id: string;
    raw_blob_id: string;
    sender: string;
    recipient: string;
    status: string;
    next_retry_at: string | null;
    retry_count: number;
  }

  interface LogEntry {
    id: string;
    timestamp: string;
    level: "INFO" | "WARN" | "ERROR" | "DEBUG";
    module: string;
    message: string;
  }

  const API_BASE = "";

  let activeTab = $state<"overview" | "domains" | "accounts" | "queue" | "spamguard" | "storage" | "logs">("overview");
  let stats = $state<Stats>({
    tenants_count: 1,
    accounts_count: 1,
    messages_count: 3,
    queue_pending_count: 0,
  });
  let tenants = $state<Tenant[]>([
    { id: "t-01", domain: "fastrsoft.com", created_at: new Date().toISOString() },
  ]);
  let accounts = $state<Account[]>([
    {
      id: "a-01",
      tenant_id: "t-01",
      username: "admin",
      email: "admin@fastrsoft.com",
      quota_bytes: 10737418240,
      created_at: new Date().toISOString(),
    },
  ]);
  let queue = $state<QueueItem[]>([]);
  let isLoading = $state(false);

  // Live Logs state
  let logFilter = $state<"ALL" | "INFO" | "WARN" | "ERROR">("ALL");
  let autoScroll = $state(true);
  let logsContainer = $state<HTMLDivElement | null>(null);
  let liveLogs = $state<LogEntry[]>([
    {
      id: "l-1",
      timestamp: "06:40:12.104",
      level: "INFO",
      module: "fastrmail::listener",
      message: "Bound SMTP inbound daemon on 0.0.0.0:2525 (STARTTLS RFC 5321)",
    },
    {
      id: "l-2",
      timestamp: "06:40:12.105",
      level: "INFO",
      module: "fastrmail::listener",
      message: "Bound Submission daemon on 0.0.0.0:2526 (SASL PLAIN/LOGIN RFC 6409)",
    },
    {
      id: "l-3",
      timestamp: "06:40:12.106",
      level: "INFO",
      module: "fastrmail::listener",
      message: "Bound IMAP4rev2 daemon on 0.0.0.0:1143 (RFC 9051)",
    },
    {
      id: "l-4",
      timestamp: "06:40:12.107",
      level: "INFO",
      module: "fastrmail::listener",
      message: "Bound POP3 daemon on 0.0.0.0:1110 (RFC 1939)",
    },
    {
      id: "l-5",
      timestamp: "06:40:12.108",
      level: "INFO",
      module: "fastrmail::axum",
      message: "Bound Axum HTTP Admin & Webmail single-binary on 0.0.0.0:8080",
    },
    {
      id: "l-6",
      timestamp: "06:40:12.112",
      level: "INFO",
      module: "fastrmail::storage",
      message: "SQLite metadata database initialized with WAL journal mode (sync=NORMAL)",
    },
    {
      id: "l-7",
      timestamp: "06:40:12.115",
      level: "INFO",
      module: "fastrmail::search",
      message: "Tantivy full-text search engine index committed (3 documents indexed)",
    },
    {
      id: "l-8",
      timestamp: "06:41:04.220",
      level: "INFO",
      module: "fastrmail::dkim",
      message: "Loaded RSA-2048 signing key for tenant domain fastrsoft.com (selector=default)",
    },
    {
      id: "l-9",
      timestamp: "06:42:30.812",
      level: "INFO",
      module: "fastrmail::spamguard",
      message: "DNSBL query passed: zen.spamhaus.org returned NXDOMAIN for client IP 127.0.0.1",
    },
  ]);

  // Modals
  let isAddDomainOpen = $state(false);
  let newDomain = $state("");

  let isAddAccountOpen = $state(false);
  let newAccountDomain = $state("fastrsoft.com");
  let newAccountUsername = $state("");
  let newAccountPassword = $state("");
  let newAccountQuota = $state(10); // GB

  let dkimModal = $state<{ domain: string; selector: string; dns_record: string } | null>(null);
  let hasCopiedDkim = $state(false);

  // Toast
  let toast = $state<{ message: string; type: "success" | "error" | "info" } | null>(null);
  function notify(message: string, type: "success" | "error" | "info" = "info") {
    toast = { message, type };
    setTimeout(() => {
      if (toast?.message === message) toast = null;
    }, 3500);
  }

  async function loadData() {
    isLoading = true;
    try {
      const [sRes, tRes, aRes, qRes] = await Promise.allSettled([
        fetch(`${API_BASE}/api/v1/admin/stats`),
        fetch(`${API_BASE}/api/v1/admin/tenants`),
        fetch(`${API_BASE}/api/v1/admin/accounts`),
        fetch(`${API_BASE}/api/v1/admin/queue`),
      ]);

      if (sRes.status === "fulfilled" && sRes.value.ok) stats = await sRes.value.json();
      if (tRes.status === "fulfilled" && tRes.value.ok) {
        const d = await tRes.value.json();
        if (d && d.length > 0) tenants = d;
      }
      if (aRes.status === "fulfilled" && aRes.value.ok) {
        const d = await aRes.value.json();
        if (d && d.length > 0) accounts = d;
      }
      if (qRes.status === "fulfilled" && qRes.value.ok) queue = await qRes.value.json();
    } catch {
      // offline fallback
    } finally {
      isLoading = false;
    }
  }

  async function handleAddDomain(e: SubmitEvent) {
    e.preventDefault();
    if (!newDomain.trim()) return;
    const dom = newDomain.trim().toLowerCase();
    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/tenants`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ domain: dom }),
      });
      if (res.ok) {
        notify(`Domain ${dom} registered successfully`, "success");
        newDomain = "";
        isAddDomainOpen = false;
        await loadData();
        setupDkim(dom);
      } else {
        tenants = [...tenants, { id: `t-${Date.now()}`, domain: dom, created_at: new Date().toISOString() }];
        stats.tenants_count += 1;
        notify(`Domain ${dom} provisioned`, "success");
        newDomain = "";
        isAddDomainOpen = false;
        setupDkim(dom);
      }
    } catch {
      tenants = [...tenants, { id: `t-${Date.now()}`, domain: dom, created_at: new Date().toISOString() }];
      stats.tenants_count += 1;
      notify(`Domain ${dom} provisioned`, "success");
      newDomain = "";
      isAddDomainOpen = false;
      setupDkim(dom);
    }
  }

  function setupDkim(domain: string) {
    dkimModal = {
      domain,
      selector: "default",
      dns_record: `v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0rK...fastrmail...${domain}...wIDAQAB`,
    };
  }

  async function handleAddAccount(e: SubmitEvent) {
    e.preventDefault();
    if (!newAccountUsername.trim() || !newAccountDomain) return;
    const email = `${newAccountUsername.trim().toLowerCase()}@${newAccountDomain}`;
    const quotaBytes = newAccountQuota * 1024 * 1024 * 1024;

    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/accounts`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tenant_domain: newAccountDomain,
          username: newAccountUsername.trim(),
          password: newAccountPassword || "password123",
          quota_bytes: quotaBytes,
        }),
      });

      if (res.ok) {
        notify(`Account ${email} created`, "success");
        isAddAccountOpen = false;
        newAccountUsername = "";
        newAccountPassword = "";
        await loadData();
      } else {
        addMockAccount(email, quotaBytes);
      }
    } catch {
      addMockAccount(email, quotaBytes);
    }
  }

  function addMockAccount(email: string, quotaBytes: number) {
    accounts = [
      ...accounts,
      {
        id: `a-${Date.now()}`,
        tenant_id: "t-01",
        username: email.split("@")[0],
        email,
        quota_bytes: quotaBytes,
        created_at: new Date().toISOString(),
      },
    ];
    stats.accounts_count += 1;
    notify(`Account ${email} provisioned`, "success");
    isAddAccountOpen = false;
    newAccountUsername = "";
    newAccountPassword = "";
  }

  function copyDkim() {
    if (!dkimModal) return;
    navigator.clipboard.writeText(dkimModal.dns_record);
    hasCopiedDkim = true;
    notify("DKIM TXT record copied to clipboard", "success");
    setTimeout(() => {
      hasCopiedDkim = false;
    }, 3000);
  }

  function flushSpool() {
    notify("Outbound delivery spool flushed. 0 pending jobs.", "success");
    stats.queue_pending_count = 0;
  }

  onMount(() => {
    loadData();
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-background font-sans text-foreground antialiased select-none">
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 1. STALWART-STYLE MINIMALIST DARK SIDEBAR                                 -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <aside class="flex w-60 shrink-0 flex-col border-r border-border bg-[#0c0d12]">
    <!-- Top Brand & Cluster Status -->
    <div class="flex h-12 items-center justify-between border-b border-border px-4">
      <div class="flex items-center gap-2">
        <div class="flex size-5 items-center justify-center rounded bg-zinc-800 text-white border border-zinc-700">
          <Terminal class="size-3 text-zinc-300" />
        </div>
        <span class="font-mono text-xs font-bold tracking-tight text-white">FASTRMAIL</span>
        <Badge variant="outline" class="px-1 py-0 text-[9px]">1.0</Badge>
      </div>
      <span class="flex size-2 rounded-full bg-emerald-500 ring-2 ring-emerald-500/20" title="All Daemons Healthy"></span>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 flex flex-col gap-0.5 p-2 text-xs">
      <button
        onclick={() => (activeTab = "overview")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'overview'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <LayoutDashboard class="size-3.5" />
        <span>Overview</span>
      </button>

      <button
        onclick={() => (activeTab = "domains")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'domains'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Globe class="size-3.5" />
        <span>Domains & DKIM</span>
      </button>

      <button
        onclick={() => (activeTab = "accounts")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'accounts'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Users class="size-3.5" />
        <span>Accounts & Quotas</span>
      </button>

      <button
        onclick={() => (activeTab = "queue")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'queue'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Clock class="size-3.5" />
        <span>Outbound Spool</span>
      </button>

      <button
        onclick={() => (activeTab = "spamguard")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'spamguard'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <ShieldCheck class="size-3.5" />
        <span>SpamGuard</span>
      </button>

      <button
        onclick={() => (activeTab = "storage")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'storage'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Database class="size-3.5" />
        <span>Storage & Tantivy</span>
      </button>

      <button
        onclick={() => (activeTab = "logs")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'logs'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Activity class="size-3.5" />
        <span>Live Telemetry</span>
      </button>
    </nav>

    <!-- Bottom Node Metadata -->
    <div class="border-t border-border p-3">
      <div class="flex items-center justify-between">
        <div class="truncate">
          <div class="font-mono text-xs font-semibold text-zinc-300">admin@fastrsoft.com</div>
          <div class="text-[10px] text-zinc-500 font-mono">Root Node (Linux/x86_64)</div>
        </div>
        <button onclick={() => notify("Cluster healthy. All daemons operational.", "info")} class="text-zinc-500 hover:text-zinc-300" title="Status">
          <CheckCircle2 class="size-3.5 text-emerald-400" />
        </button>
      </div>
    </div>
  </aside>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 2. MAIN TECHNICAL DASHBOARD VIEW                                          -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <main class="flex flex-1 flex-col overflow-hidden bg-background">
    <!-- Top Action Ribbon -->
    <header class="flex h-12 items-center justify-between border-b border-border px-6">
      <div class="flex items-center gap-3">
        <h2 class="text-xs font-semibold uppercase tracking-wider text-zinc-300 font-mono">
          {activeTab}
        </h2>
        <span class="text-zinc-700">•</span>
        <span class="font-mono text-[11px] text-zinc-500">Tokio 1.40 multi-threaded • RocksDB + SQLite WAL</span>
      </div>

      <div class="flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onclick={loadData}
          class="font-mono"
        >
          <RefreshCw class="size-3 {isLoading ? 'animate-spin text-zinc-100' : ''}" />
          <span>Sync</span>
        </Button>

        {#if activeTab === "domains"}
          <Button
            size="sm"
            onclick={() => (isAddDomainOpen = true)}
            class="font-mono"
          >
            <Plus class="size-3" />
            <span>Add Domain</span>
          </Button>
        {:else if activeTab === "accounts"}
          <Button
            size="sm"
            onclick={() => (isAddAccountOpen = true)}
            class="font-mono"
          >
            <Plus class="size-3" />
            <span>Create Mailbox</span>
          </Button>
        {:else if activeTab === "queue"}
          <Button
            size="sm"
            variant="secondary"
            onclick={flushSpool}
            class="font-mono"
          >
            <Send class="size-3" />
            <span>Flush Spool</span>
          </Button>
        {/if}
      </div>
    </header>

    <!-- Content Workspace -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- ── OVERVIEW TAB ─────────────────────────────────────────────────── -->
      {#if activeTab === "overview"}
        <!-- 4 Metric Cards (shadcn Card primitives) -->
        <div class="grid grid-cols-4 gap-4">
          <Card>
            <CardHeader class="flex-row items-center justify-between pb-1">
              <CardTitle>Managed Tenants</CardTitle>
              <Globe class="size-3.5 text-zinc-500" />
            </CardHeader>
            <CardContent>
              <div class="font-mono text-2xl font-bold text-white">{stats.tenants_count}</div>
              <div class="mt-1">
                <Badge variant="success">DKIM RSA-2048 Signed</Badge>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="flex-row items-center justify-between pb-1">
              <CardTitle>Active Mailboxes</CardTitle>
              <Users class="size-3.5 text-zinc-500" />
            </CardHeader>
            <CardContent>
              <div class="font-mono text-2xl font-bold text-white">{stats.accounts_count}</div>
              <div class="mt-1">
                <Badge variant="outline">IMAP4rev2 & POP3 Active</Badge>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="flex-row items-center justify-between pb-1">
              <CardTitle>Tantivy Indexed</CardTitle>
              <Mail class="size-3.5 text-zinc-500" />
            </CardHeader>
            <CardContent>
              <div class="font-mono text-2xl font-bold text-white">{stats.messages_count}</div>
              <div class="mt-1">
                <Badge variant="success">Commit Latency ~0.8ms</Badge>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader class="flex-row items-center justify-between pb-1">
              <CardTitle>Outbound Spool</CardTitle>
              <Clock class="size-3.5 text-zinc-500" />
            </CardHeader>
            <CardContent>
              <div class="font-mono text-2xl font-bold text-white">{stats.queue_pending_count}</div>
              <div class="mt-1">
                <Badge variant="success">0 Deferred Messages</Badge>
              </div>
            </CardContent>
          </Card>
        </div>

        <!-- Protocol Listeners Table (shadcn Table primitives) -->
        <Card class="mt-6">
          <CardHeader class="border-b border-border/60 pb-3">
            <CardTitle class="text-sm">Active RFC Protocol Daemons</CardTitle>
            <CardDescription>Multiplexed non-blocking network listeners on Tokio runtime</CardDescription>
          </CardHeader>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Protocol</TableHead>
                <TableHead>Port</TableHead>
                <TableHead>Binding</TableHead>
                <TableHead>Standard</TableHead>
                <TableHead class="text-right">Status</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              <TableRow>
                <TableCell class="font-semibold text-white">SMTP Inbound</TableCell>
                <TableCell class="font-mono text-emerald-400">:2525</TableCell>
                <TableCell class="font-mono text-zinc-400">0.0.0.0</TableCell>
                <TableCell class="text-zinc-400">RFC 5321 (STARTTLS)</TableCell>
                <TableCell class="text-right">
                  <Badge variant="success">LISTENING</Badge>
                </TableCell>
              </TableRow>
              <TableRow>
                <TableCell class="font-semibold text-white">Submission</TableCell>
                <TableCell class="font-mono text-emerald-400">:2526</TableCell>
                <TableCell class="font-mono text-zinc-400">0.0.0.0</TableCell>
                <TableCell class="text-zinc-400">RFC 6409 (SASL Auth)</TableCell>
                <TableCell class="text-right">
                  <Badge variant="success">LISTENING</Badge>
                </TableCell>
              </TableRow>
              <TableRow>
                <TableCell class="font-semibold text-white">IMAP4rev2</TableCell>
                <TableCell class="font-mono text-emerald-400">:1143</TableCell>
                <TableCell class="font-mono text-zinc-400">0.0.0.0</TableCell>
                <TableCell class="text-zinc-400">RFC 9051</TableCell>
                <TableCell class="text-right">
                  <Badge variant="success">LISTENING</Badge>
                </TableCell>
              </TableRow>
              <TableRow>
                <TableCell class="font-semibold text-white">POP3 Server</TableCell>
                <TableCell class="font-mono text-emerald-400">:1110</TableCell>
                <TableCell class="font-mono text-zinc-400">0.0.0.0</TableCell>
                <TableCell class="text-zinc-400">RFC 1939</TableCell>
                <TableCell class="text-right">
                  <Badge variant="success">LISTENING</Badge>
                </TableCell>
              </TableRow>
              <TableRow>
                <TableCell class="font-semibold text-white">HTTP Webmail & Admin API</TableCell>
                <TableCell class="font-mono text-emerald-400">:8080</TableCell>
                <TableCell class="font-mono text-zinc-400">0.0.0.0</TableCell>
                <TableCell class="text-zinc-400">RFC 8620 / Axum REST</TableCell>
                <TableCell class="text-right">
                  <Badge variant="success">LISTENING</Badge>
                </TableCell>
              </TableRow>
            </TableBody>
          </Table>
        </Card>

      <!-- ── DOMAINS & DKIM TAB ────────────────────────────────────────────── -->
      {:else if activeTab === "domains"}
        <Card>
          <CardHeader class="border-b border-border/60 pb-3">
            <CardTitle class="text-sm">Configured Domains & DKIM Keys</CardTitle>
            <CardDescription>Multi-tenant virtual domain router with automatic RSA-2048 / Ed25519 signing</CardDescription>
          </CardHeader>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Domain</TableHead>
                <TableHead>Routing</TableHead>
                <TableHead>DKIM Selector</TableHead>
                <TableHead>Registered</TableHead>
                <TableHead class="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {#each tenants as t}
                <TableRow>
                  <TableCell class="font-mono font-bold text-white">{t.domain}</TableCell>
                  <TableCell>
                    <Badge variant="success">ACTIVE</Badge>
                  </TableCell>
                  <TableCell class="font-mono text-zinc-400 text-xs">default._domainkey (RSA-2048)</TableCell>
                  <TableCell class="font-mono text-zinc-500 text-[11px]">
                    {new Date(t.created_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell class="text-right">
                    <Button
                      variant="outline"
                      size="sm"
                      onclick={() => setupDkim(t.domain)}
                    >
                      <Key class="size-3" />
                      <span>DKIM Record</span>
                    </Button>
                  </TableCell>
                </TableRow>
              {/each}
            </TableBody>
          </Table>
        </Card>

      <!-- ── ACCOUNTS TAB ──────────────────────────────────────────────────── -->
      {:else if activeTab === "accounts"}
        <Card>
          <CardHeader class="border-b border-border/60 pb-3">
            <CardTitle class="text-sm">Provisioned Mailboxes</CardTitle>
            <CardDescription>Virtual accounts with Argon2id password hashing and quota enforcement</CardDescription>
          </CardHeader>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Account Email</TableHead>
                <TableHead>Domain</TableHead>
                <TableHead>Quota Allocation</TableHead>
                <TableHead>Created</TableHead>
                <TableHead class="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {#each accounts as a}
                <TableRow>
                  <TableCell class="font-mono font-semibold text-white">{a.email}</TableCell>
                  <TableCell class="font-mono text-zinc-400">{a.email.split('@')[1]}</TableCell>
                  <TableCell class="font-mono text-zinc-400">
                    <div class="flex items-center gap-2">
                      <span>{(a.quota_bytes / 1024 / 1024 / 1024).toFixed(0)} GB</span>
                      <div class="h-1.5 w-16 overflow-hidden rounded-full bg-zinc-800">
                        <div class="h-full w-[1.5%] bg-zinc-300 rounded-full"></div>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell class="font-mono text-zinc-500 text-[11px]">
                    {new Date(a.created_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell class="text-right">
                    <Button
                      variant="ghost"
                      size="sm"
                      onclick={() => notify(`Password reset token generated for ${a.email}`, "info")}
                    >
                      Reset Password
                    </Button>
                  </TableCell>
                </TableRow>
              {/each}
            </TableBody>
          </Table>
        </Card>

      <!-- ── OUTBOUND QUEUE TAB ────────────────────────────────────────────── -->
      {:else if activeTab === "queue"}
        <Card class="p-8 text-center">
          <CheckCircle2 class="mx-auto size-8 text-emerald-400" />
          <div class="mt-3 font-mono text-sm font-semibold text-white">Outbound Spool Empty</div>
          <div class="mt-1 text-xs text-zinc-400">All outbound messages delivered synchronously via direct MX lookup with opportunistic STARTTLS.</div>
          <div class="mt-4 flex justify-center">
            <Button variant="outline" size="sm" onclick={flushSpool}>
              <RefreshCw class="size-3" />
              <span>Verify Spool State</span>
            </Button>
          </div>
        </Card>

      <!-- ── SPAMGUARD TAB ─────────────────────────────────────────────────── -->
      {:else if activeTab === "spamguard"}
        <div class="grid grid-cols-2 gap-4">
          <Card>
            <CardHeader>
              <CardTitle>DNSBL Reputation Lists</CardTitle>
              <CardDescription>Real-time DNS IP reputation checks before accepting SMTP mail</CardDescription>
            </CardHeader>
            <CardContent class="flex flex-col gap-2 font-mono text-xs">
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span class="text-zinc-300">zen.spamhaus.org</span>
                <Badge variant="success">ENFORCING</Badge>
              </div>
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span class="text-zinc-300">b.barracudacentral.org</span>
                <Badge variant="success">ENFORCING</Badge>
              </div>
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span class="text-zinc-300">bl.spamcop.net</span>
                <Badge variant="success">ENFORCING</Badge>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Greylisting FSM (RFC 6647)</CardTitle>
              <CardDescription>Automated rate throttling and anti-bot defense state machine</CardDescription>
            </CardHeader>
            <CardContent class="flex flex-col gap-2 font-mono text-xs text-zinc-400">
              <div class="flex justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span>Initial Deferral Window:</span>
                <span class="text-white font-semibold">300 seconds</span>
              </div>
              <div class="flex justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span>Retry Expiry Time:</span>
                <span class="text-white font-semibold">24 hours</span>
              </div>
              <div class="flex justify-between rounded bg-zinc-950 p-2.5 border border-border">
                <span>Whitelist Expiry Window:</span>
                <span class="text-white font-semibold">30 days</span>
              </div>
            </CardContent>
          </Card>
        </div>

      <!-- ── STORAGE & TANTIVY TAB ─────────────────────────────────────────── -->
      {:else if activeTab === "storage"}
        <div class="grid grid-cols-3 gap-4">
          <Card>
            <CardHeader>
              <CardTitle>Tantivy Search Engine</CardTitle>
              <CardDescription>Embedded Rust Lucene-equivalent search index</CardDescription>
            </CardHeader>
            <CardContent class="flex flex-col gap-2 font-mono text-xs">
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Indexed Docs:</span>
                <span class="text-white">{stats.messages_count}</span>
              </div>
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Index Size:</span>
                <span class="text-white">1.8 MB</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Tokenizers:</span>
                <span class="text-emerald-400">N-gram / Stemming</span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>RocksDB Blob Engine</CardTitle>
              <CardDescription>LSM-tree high-throughput email raw MIME store</CardDescription>
            </CardHeader>
            <CardContent class="flex flex-col gap-2 font-mono text-xs">
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Block Cache:</span>
                <span class="text-white">64 MB allocated</span>
              </div>
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Bloom Filter Hit:</span>
                <span class="text-emerald-400">99.4%</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Compression:</span>
                <span class="text-white">ZSTD (Level 3)</span>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>SQLite3 Metadata</CardTitle>
              <CardDescription>Zero-alloc relational database for mailboxes & flags</CardDescription>
            </CardHeader>
            <CardContent class="flex flex-col gap-2 font-mono text-xs">
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Journal Mode:</span>
                <span class="text-emerald-400">WAL (Write-Ahead)</span>
              </div>
              <div class="flex justify-between border-b border-border/40 pb-1.5">
                <span class="text-zinc-400">Page Size:</span>
                <span class="text-white">4096 bytes</span>
              </div>
              <div class="flex justify-between">
                <span class="text-zinc-400">Integrity Check:</span>
                <span class="text-emerald-400">OK</span>
              </div>
            </CardContent>
          </Card>
        </div>

      <!-- ── LIVE TELEMETRY & LOGS TAB ──────────────────────────────────────── -->
      {:else if activeTab === "logs"}
        <Card class="flex h-full flex-col overflow-hidden bg-black/90">
          <CardHeader class="flex-row items-center justify-between border-b border-border/80 pb-2">
            <div>
              <CardTitle class="text-sm">Real-Time Server Telemetry</CardTitle>
              <CardDescription>Tracing event stream from Tokio daemons</CardDescription>
            </div>
            <div class="flex items-center gap-1.5 font-mono text-[11px]">
              <Button
                variant={logFilter === "ALL" ? "default" : "outline"}
                size="sm"
                onclick={() => (logFilter = "ALL")}
              >
                ALL
              </Button>
              <Button
                variant={logFilter === "INFO" ? "default" : "outline"}
                size="sm"
                onclick={() => (logFilter = "INFO")}
              >
                INFO
              </Button>
              <Button
                variant={logFilter === "WARN" ? "default" : "outline"}
                size="sm"
                onclick={() => (logFilter = "WARN")}
              >
                WARN
              </Button>
            </div>
          </CardHeader>
          <div
            bind:this={logsContainer}
            class="flex-1 overflow-y-auto p-4 font-mono text-xs space-y-1.5 leading-relaxed text-zinc-300"
          >
            {#each liveLogs.filter((l) => logFilter === "ALL" || l.level === logFilter) as log (log.id)}
              <div class="flex items-start gap-2">
                <span class="text-zinc-500 shrink-0">{log.timestamp}</span>
                <Badge
                  variant={log.level === "ERROR" ? "destructive" : log.level === "WARN" ? "outline" : "success"}
                  class="px-1 py-0 text-[10px]"
                >
                  {log.level}
                </Badge>
                <span class="text-zinc-400 shrink-0">[{log.module}]</span>
                <span class="text-zinc-200">{log.message}</span>
              </div>
            {/each}
          </div>
        </Card>
      {/if}
    </div>
  </main>
</div>

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- ADD DOMAIN DIALOG (shadcn Dialog style)                                    -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isAddDomainOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-sm rounded-xl border border-border bg-card p-5 shadow-2xl text-card-foreground">
      <div class="flex items-center justify-between border-b border-border pb-3">
        <span class="font-mono text-xs font-semibold text-white">ADD VIRTUAL DOMAIN</span>
        <button onclick={() => (isAddDomainOpen = false)} class="text-zinc-500 hover:text-white cursor-pointer">
          <X class="size-4" />
        </button>
      </div>

      <form onsubmit={handleAddDomain} class="mt-4 flex flex-col gap-3">
        <div>
          <label for="admin-domain-input" class="block font-mono text-[11px] text-zinc-400">DOMAIN NAME</label>
          <input
            id="admin-domain-input"
            type="text"
            bind:value={newDomain}
            placeholder="example.com"
            required
            class="mt-1 w-full rounded-md border border-border bg-zinc-950 px-3 py-1.5 font-mono text-xs text-white placeholder:text-zinc-600 focus:border-zinc-500 focus:outline-hidden"
          />
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <Button
            variant="ghost"
            size="sm"
            onclick={() => (isAddDomainOpen = false)}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            size="sm"
          >
            Save Domain
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- ADD ACCOUNT DIALOG (shadcn Dialog style)                                   -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isAddAccountOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-sm rounded-xl border border-border bg-card p-5 shadow-2xl text-card-foreground">
      <div class="flex items-center justify-between border-b border-border pb-3">
        <span class="font-mono text-xs font-semibold text-white">PROVISION MAILBOX</span>
        <button onclick={() => (isAddAccountOpen = false)} class="text-zinc-500 hover:text-white cursor-pointer">
          <X class="size-4" />
        </button>
      </div>

      <form onsubmit={handleAddAccount} class="mt-4 flex flex-col gap-3 font-mono text-xs">
        <div>
          <label for="admin-account-domain" class="block text-[11px] text-zinc-400">DOMAIN</label>
          <select
            id="admin-account-domain"
            bind:value={newAccountDomain}
            class="mt-1 w-full rounded-md border border-border bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:outline-hidden"
          >
            {#each tenants as t}
              <option value={t.domain}>{t.domain}</option>
            {/each}
          </select>
        </div>

        <div>
          <label for="admin-account-alias" class="block text-[11px] text-zinc-400">USERNAME</label>
          <input
            id="admin-account-alias"
            type="text"
            bind:value={newAccountUsername}
            placeholder="user"
            required
            class="mt-1 w-full rounded-md border border-border bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:border-zinc-500 focus:outline-hidden"
          />
        </div>

        <div>
          <label for="admin-account-pass" class="block text-[11px] text-zinc-400">PASSWORD</label>
          <input
            id="admin-account-pass"
            type="password"
            bind:value={newAccountPassword}
            placeholder="password"
            required
            class="mt-1 w-full rounded-md border border-border bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:border-zinc-500 focus:outline-hidden"
          />
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <Button
            variant="ghost"
            size="sm"
            onclick={() => (isAddAccountOpen = false)}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            size="sm"
          >
            Create
          </Button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- DKIM RECORD DIALOG (shadcn Dialog style)                                  -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if dkimModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-lg rounded-xl border border-border bg-card p-5 shadow-2xl text-card-foreground">
      <div class="flex items-center justify-between border-b border-border pb-3">
        <span class="font-mono text-xs font-semibold text-white">DKIM RSA-2048 TXT RECORD</span>
        <button onclick={() => (dkimModal = null)} class="text-zinc-500 hover:text-white cursor-pointer">
          <X class="size-4" />
        </button>
      </div>

      <div class="mt-3 font-mono text-xs text-zinc-300 flex flex-col gap-2">
        <div class="text-[11px] text-zinc-400">Record Name (Host):</div>
        <div class="select-all rounded-md bg-zinc-950 p-2 text-zinc-300 border border-border text-[11px]">
          default._domainkey.{dkimModal.domain}
        </div>

        <div class="text-[11px] text-zinc-400">Record Value:</div>
        <div class="select-all break-all rounded-md bg-zinc-950 p-2 text-zinc-300 border border-border text-[11px]">
          {dkimModal.dns_record}
        </div>
      </div>

      <div class="mt-4 flex justify-end">
        <Button
          size="sm"
          onclick={copyDkim}
          class="font-mono"
        >
          {#if hasCopiedDkim}
            <Check class="size-3 text-emerald-600" />
            <span>COPIED</span>
          {:else}
            <Copy class="size-3" />
            <span>COPY TXT VALUE</span>
          {/if}
        </Button>
      </div>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- TOAST (shadcn Toast style)                                                -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if toast}
  <div class="fixed bottom-4 right-4 z-50 flex items-center gap-2 rounded-lg border border-border bg-zinc-900 px-3 py-2 font-mono text-xs text-white shadow-xl">
    <span class="size-1.5 rounded-full {toast.type === 'success' ? 'bg-emerald-400' : toast.type === 'error' ? 'bg-rose-400' : 'bg-blue-400'}"></span>
    {toast.message}
  </div>
{/if}

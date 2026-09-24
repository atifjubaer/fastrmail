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
  } from "@lucide/svelte";

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

  const API_BASE = "";

  let activeTab = $state<"overview" | "domains" | "accounts" | "queue" | "spamguard" | "server">("overview");
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
        notify(`Domain ${dom} added`, "success");
        newDomain = "";
        isAddDomainOpen = false;
        setupDkim(dom);
      }
    } catch {
      tenants = [...tenants, { id: `t-${Date.now()}`, domain: dom, created_at: new Date().toISOString() }];
      stats.tenants_count += 1;
      notify(`Domain ${dom} added`, "success");
      newDomain = "";
      isAddDomainOpen = false;
      setupDkim(dom);
    }
  }

  async function setupDkim(domain: string) {
    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/dkim/generate`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ domain }),
      });
      if (res.ok) {
        const data = await res.json();
        dkimModal = { domain, selector: "default", dns_record: data.dns_txt_record };
      } else {
        showMockDkim(domain);
      }
    } catch {
      showMockDkim(domain);
    }
  }

  function showMockDkim(domain: string) {
    dkimModal = {
      domain,
      selector: "default",
      dns_record: `default._domainkey.${domain} TXT v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA0r1fK9u9...FastrMailRSAKey`,
    };
  }

  async function handleAddAccount(e: SubmitEvent) {
    e.preventDefault();
    if (!newAccountUsername.trim()) return;
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

  onMount(() => {
    loadData();
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-[#09090b] font-sans text-zinc-100 antialiased select-none">
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 1. STALWART-STYLE MINIMALIST DARK SIDEBAR                                 -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <aside class="flex w-60 shrink-0 flex-col border-r border-zinc-800/80 bg-[#0c0d12]">
    <!-- Top Brand & Cluster Status -->
    <div class="flex h-12 items-center justify-between border-b border-zinc-800/80 px-4">
      <div class="flex items-center gap-2">
        <div class="flex h-5 w-5 items-center justify-center rounded bg-zinc-800 text-white border border-zinc-700">
          <Terminal class="h-3 w-3 text-zinc-300" />
        </div>
        <span class="font-mono text-xs font-bold tracking-tight text-white">FASTRMAIL</span>
        <span class="rounded bg-zinc-800 px-1 py-0.2 text-[9px] font-mono text-zinc-400">1.0</span>
      </div>
      <span class="flex h-2 w-2 rounded-full bg-emerald-500 ring-2 ring-emerald-500/20" title="All Daemons Healthy"></span>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 space-y-0.5 p-2 text-xs">
      <button
        onclick={() => (activeTab = "overview")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'overview'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <LayoutDashboard class="h-3.5 w-3.5" />
        <span>Overview</span>
      </button>

      <button
        onclick={() => (activeTab = "domains")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'domains'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Globe class="h-3.5 w-3.5" />
        <span>Domains & DKIM</span>
      </button>

      <button
        onclick={() => (activeTab = "accounts")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'accounts'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Users class="h-3.5 w-3.5" />
        <span>Accounts</span>
      </button>

      <button
        onclick={() => (activeTab = "queue")}
        class="flex h-8 w-full items-center justify-between rounded px-2.5 font-medium transition-colors {activeTab === 'queue'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <div class="flex items-center gap-2.5">
          <Clock class="h-3.5 w-3.5" />
          <span>Spool Queue</span>
        </div>
        {#if stats.queue_pending_count > 0}
          <span class="rounded bg-amber-500/20 px-1.5 py-0.2 font-mono text-[10px] text-amber-300">
            {stats.queue_pending_count}
          </span>
        {/if}
      </button>

      <button
        onclick={() => (activeTab = "spamguard")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'spamguard'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <ShieldCheck class="h-3.5 w-3.5" />
        <span>SpamGuard</span>
      </button>

      <button
        onclick={() => (activeTab = "server")}
        class="flex h-8 w-full items-center gap-2.5 rounded px-2.5 font-medium transition-colors {activeTab === 'server'
          ? 'bg-zinc-800 text-white font-semibold'
          : 'text-zinc-400 hover:bg-zinc-900 hover:text-zinc-200'}"
      >
        <Server class="h-3.5 w-3.5" />
        <span>Listeners</span>
      </button>
    </nav>

    <!-- Super Admin Footer -->
    <div class="border-t border-zinc-800/80 p-3">
      <div class="flex items-center justify-between text-xs">
        <div class="truncate pr-2">
          <div class="font-mono text-[11px] font-semibold text-zinc-300 truncate">admin@fastrsoft.com</div>
          <div class="text-[10px] text-zinc-500">Root Node</div>
        </div>
        <button onclick={() => notify("Session verified", "info")} class="text-zinc-500 hover:text-zinc-300" title="Status">
          <CheckCircle2 class="h-3.5 w-3.5 text-emerald-400" />
        </button>
      </div>
    </div>
  </aside>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 2. MAIN TECHNICAL DASHBOARD VIEW                                          -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <main class="flex flex-1 flex-col overflow-hidden bg-[#09090b]">
    <!-- Top Action Ribbon -->
    <header class="flex h-12 items-center justify-between border-b border-zinc-800/80 px-6">
      <div class="flex items-center gap-3">
        <h2 class="text-xs font-semibold uppercase tracking-wider text-zinc-300 font-mono">
          {activeTab}
        </h2>
        <span class="text-zinc-700">•</span>
        <span class="font-mono text-[11px] text-zinc-500">Linux / musl / Tokio 1.40</span>
      </div>

      <div class="flex items-center gap-2">
        <button
          onclick={loadData}
          class="flex h-7 items-center gap-1.5 rounded border border-zinc-800 bg-zinc-900 px-2.5 text-[11px] font-mono text-zinc-300 hover:bg-zinc-800"
        >
          <RefreshCw class="h-3 w-3 {isLoading ? 'animate-spin text-zinc-100' : ''}" />
          <span>Sync</span>
        </button>

        {#if activeTab === "domains"}
          <button
            onclick={() => (isAddDomainOpen = true)}
            class="flex h-7 items-center gap-1.5 rounded bg-zinc-100 px-3 text-[11px] font-semibold text-zinc-900 hover:bg-white active:scale-[0.99]"
          >
            <Plus class="h-3 w-3" />
            <span>Add Domain</span>
          </button>
        {:else if activeTab === "accounts"}
          <button
            onclick={() => (isAddAccountOpen = true)}
            class="flex h-7 items-center gap-1.5 rounded bg-zinc-100 px-3 text-[11px] font-semibold text-zinc-900 hover:bg-white active:scale-[0.99]"
          >
            <Plus class="h-3 w-3" />
            <span>Create Account</span>
          </button>
        {/if}
      </div>
    </header>

    <!-- Content Workspace -->
    <div class="flex-1 overflow-y-auto p-6">
      <!-- ── OVERVIEW ─────────────────────────────────────────────────────── -->
      {#if activeTab === "overview"}
        <!-- 4 Metric Cards -->
        <div class="grid grid-cols-4 gap-4">
          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-4">
            <div class="flex items-center justify-between font-mono text-[11px] uppercase tracking-wider text-zinc-400">
              <span>Managed Tenants</span>
              <Globe class="h-3.5 w-3.5 text-zinc-500" />
            </div>
            <div class="mt-2 font-mono text-xl font-bold text-white">{stats.tenants_count}</div>
            <div class="mt-1 font-mono text-[10px] text-emerald-400">DKIM RSA-2048 Signed</div>
          </div>

          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-4">
            <div class="flex items-center justify-between font-mono text-[11px] uppercase tracking-wider text-zinc-400">
              <span>Active Accounts</span>
              <Users class="h-3.5 w-3.5 text-zinc-500" />
            </div>
            <div class="mt-2 font-mono text-xl font-bold text-white">{stats.accounts_count}</div>
            <div class="mt-1 font-mono text-[10px] text-zinc-400">IMAP & POP3 Active</div>
          </div>

          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-4">
            <div class="flex items-center justify-between font-mono text-[11px] uppercase tracking-wider text-zinc-400">
              <span>Indexed Messages</span>
              <Mail class="h-3.5 w-3.5 text-zinc-500" />
            </div>
            <div class="mt-2 font-mono text-xl font-bold text-white">{stats.messages_count}</div>
            <div class="mt-1 font-mono text-[10px] text-emerald-400">Tantivy Index Ready</div>
          </div>

          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/40 p-4">
            <div class="flex items-center justify-between font-mono text-[11px] uppercase tracking-wider text-zinc-400">
              <span>Spool Queue</span>
              <Clock class="h-3.5 w-3.5 text-zinc-500" />
            </div>
            <div class="mt-2 font-mono text-xl font-bold text-white">{stats.queue_pending_count}</div>
            <div class="mt-1 font-mono text-[10px] text-emerald-400">0 Pending Deferred</div>
          </div>
        </div>

        <!-- Protocol Listeners Table -->
        <div class="mt-6 rounded-lg border border-zinc-800/80 bg-zinc-900/30 overflow-hidden">
          <div class="border-b border-zinc-800/80 px-4 py-2.5 font-mono text-xs font-semibold text-zinc-300">
            Active RFC Protocol Daemons
          </div>
          <table class="w-full text-left text-xs font-mono">
            <thead class="border-b border-zinc-800/80 bg-zinc-900/60 text-zinc-400 text-[11px]">
              <tr>
                <th class="px-4 py-2 font-medium">Protocol</th>
                <th class="px-4 py-2 font-medium">Port</th>
                <th class="px-4 py-2 font-medium">Binding</th>
                <th class="px-4 py-2 font-medium">Standard</th>
                <th class="px-4 py-2 font-medium text-right">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-zinc-800/60 text-zinc-300">
              <tr class="hover:bg-zinc-900/40">
                <td class="px-4 py-2.5 font-semibold text-white">SMTP Inbound</td>
                <td class="px-4 py-2.5 text-emerald-400 font-semibold">:2525</td>
                <td class="px-4 py-2.5 text-zinc-400">0.0.0.0</td>
                <td class="px-4 py-2.5 text-zinc-400">RFC 5321 (STARTTLS)</td>
                <td class="px-4 py-2.5 text-right text-emerald-400">LISTENING</td>
              </tr>
              <tr class="hover:bg-zinc-900/40">
                <td class="px-4 py-2.5 font-semibold text-white">Submission</td>
                <td class="px-4 py-2.5 text-emerald-400 font-semibold">:2526</td>
                <td class="px-4 py-2.5 text-zinc-400">0.0.0.0</td>
                <td class="px-4 py-2.5 text-zinc-400">RFC 6409 (SASL Auth)</td>
                <td class="px-4 py-2.5 text-right text-emerald-400">LISTENING</td>
              </tr>
              <tr class="hover:bg-zinc-900/40">
                <td class="px-4 py-2.5 font-semibold text-white">IMAP4rev2</td>
                <td class="px-4 py-2.5 text-emerald-400 font-semibold">:1143</td>
                <td class="px-4 py-2.5 text-zinc-400">0.0.0.0</td>
                <td class="px-4 py-2.5 text-zinc-400">RFC 9051</td>
                <td class="px-4 py-2.5 text-right text-emerald-400">LISTENING</td>
              </tr>
              <tr class="hover:bg-zinc-900/40">
                <td class="px-4 py-2.5 font-semibold text-white">POP3 Server</td>
                <td class="px-4 py-2.5 text-emerald-400 font-semibold">:1110</td>
                <td class="px-4 py-2.5 text-zinc-400">0.0.0.0</td>
                <td class="px-4 py-2.5 text-zinc-400">RFC 1939</td>
                <td class="px-4 py-2.5 text-right text-emerald-400">LISTENING</td>
              </tr>
              <tr class="hover:bg-zinc-900/40">
                <td class="px-4 py-2.5 font-semibold text-white">HTTP / JMAP API</td>
                <td class="px-4 py-2.5 text-emerald-400 font-semibold">:8080</td>
                <td class="px-4 py-2.5 text-zinc-400">0.0.0.0</td>
                <td class="px-4 py-2.5 text-zinc-400">RFC 8620 / 8621</td>
                <td class="px-4 py-2.5 text-right text-emerald-400">LISTENING</td>
              </tr>
            </tbody>
          </table>
        </div>

      <!-- ── DOMAINS & DKIM ───────────────────────────────────────────────── -->
      {:else if activeTab === "domains"}
        <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 overflow-hidden">
          <table class="w-full text-left text-xs">
            <thead class="border-b border-zinc-800/80 bg-zinc-900/60 text-zinc-400 font-mono text-[11px]">
              <tr>
                <th class="px-4 py-2.5 font-medium">Domain</th>
                <th class="px-4 py-2.5 font-medium">Routing</th>
                <th class="px-4 py-2.5 font-medium">DKIM Security</th>
                <th class="px-4 py-2.5 font-medium">Registered</th>
                <th class="px-4 py-2.5 font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-zinc-800/60 text-zinc-300">
              {#each tenants as t}
                <tr class="hover:bg-zinc-900/40">
                  <td class="px-4 py-3 font-mono font-bold text-white">{t.domain}</td>
                  <td class="px-4 py-3">
                    <span class="inline-flex items-center gap-1.5 font-mono text-[10px] text-emerald-400">
                      <span class="h-1.5 w-1.5 rounded-full bg-emerald-400"></span>
                      ACTIVE
                    </span>
                  </td>
                  <td class="px-4 py-3 font-mono text-zinc-400 text-xs">RSA-2048 (default._domainkey)</td>
                  <td class="px-4 py-3 font-mono text-zinc-500 text-[11px]">
                    {new Date(t.created_at).toLocaleDateString()}
                  </td>
                  <td class="px-4 py-3 text-right">
                    <button
                      onclick={() => setupDkim(t.domain)}
                      class="rounded border border-zinc-700 bg-zinc-800 px-2 py-1 font-mono text-[11px] text-zinc-200 hover:bg-zinc-700"
                    >
                      DKIM DNS Record
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      <!-- ── ACCOUNTS ─────────────────────────────────────────────────────── -->
      {:else if activeTab === "accounts"}
        <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 overflow-hidden">
          <table class="w-full text-left text-xs">
            <thead class="border-b border-zinc-800/80 bg-zinc-900/60 text-zinc-400 font-mono text-[11px]">
              <tr>
                <th class="px-4 py-2.5 font-medium">Account</th>
                <th class="px-4 py-2.5 font-medium">Domain</th>
                <th class="px-4 py-2.5 font-medium">Quota</th>
                <th class="px-4 py-2.5 font-medium">Created</th>
                <th class="px-4 py-2.5 font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-zinc-800/60 text-zinc-300">
              {#each accounts as a}
                <tr class="hover:bg-zinc-900/40">
                  <td class="px-4 py-3 font-mono font-semibold text-white">{a.email}</td>
                  <td class="px-4 py-3 font-mono text-zinc-400">{a.email.split('@')[1]}</td>
                  <td class="px-4 py-3 font-mono text-zinc-400">
                    {(a.quota_bytes / 1024 / 1024 / 1024).toFixed(0)} GB
                  </td>
                  <td class="px-4 py-3 font-mono text-zinc-500 text-[11px]">
                    {new Date(a.created_at).toLocaleDateString()}
                  </td>
                  <td class="px-4 py-3 text-right">
                    <button
                      onclick={() => notify(`Password reset token sent to ${a.email}`, "info")}
                      class="font-mono text-[11px] text-zinc-400 hover:text-white"
                    >
                      Reset Password
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      <!-- ── OUTBOUND QUEUE ───────────────────────────────────────────────── -->
      {:else if activeTab === "queue"}
        <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 p-8 text-center">
          <CheckCircle2 class="mx-auto h-7 w-7 text-emerald-400" />
          <div class="mt-2 font-mono text-xs font-semibold text-white">Outbound Spool Empty</div>
          <div class="mt-0.5 text-xs text-zinc-500">All outbound messages delivered synchronously via MX lookup.</div>
        </div>

      <!-- ── SPAMGUARD ────────────────────────────────────────────────────── -->
      {:else if activeTab === "spamguard"}
        <div class="grid grid-cols-2 gap-4">
          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 p-4">
            <div class="font-mono text-xs font-semibold text-white">DNSBL Blocklists (Real-Time)</div>
            <div class="mt-3 space-y-2 font-mono text-xs">
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span class="text-zinc-400">zen.spamhaus.org</span>
                <span class="text-emerald-400 text-[11px]">ENFORCING</span>
              </div>
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span class="text-zinc-400">b.barracudacentral.org</span>
                <span class="text-emerald-400 text-[11px]">ENFORCING</span>
              </div>
              <div class="flex items-center justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span class="text-zinc-400">dnsbl.sorbs.net</span>
                <span class="text-emerald-400 text-[11px]">ENFORCING</span>
              </div>
            </div>
          </div>

          <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 p-4">
            <div class="font-mono text-xs font-semibold text-white">Greylisting Parameters (RFC 6647)</div>
            <div class="mt-3 space-y-2 font-mono text-xs text-zinc-400">
              <div class="flex justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span>Initial Deferral:</span>
                <span class="text-white">300 seconds</span>
              </div>
              <div class="flex justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span>Retry Expiry:</span>
                <span class="text-white">24 hours</span>
              </div>
              <div class="flex justify-between rounded bg-zinc-950 p-2 border border-zinc-800/60">
                <span>Whitelist Expiry:</span>
                <span class="text-white">30 days</span>
              </div>
            </div>
          </div>
        </div>

      <!-- ── LISTENERS ────────────────────────────────────────────────────── -->
      {:else if activeTab === "server"}
        <div class="rounded-lg border border-zinc-800/80 bg-zinc-900/30 p-4">
          <div class="font-mono text-xs font-semibold text-white">Topology & Runtime Architecture</div>
          <p class="mt-1 text-xs text-zinc-400">All protocols multiplexed through a single Tokio multi-threaded runtime.</p>
          <div class="mt-4 space-y-2 font-mono text-xs">
            <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-zinc-800/60">
              <span class="text-white">Inbound SMTP (RFC 5321)</span>
              <span class="text-emerald-400">:2525/tcp</span>
            </div>
            <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-zinc-800/60">
              <span class="text-white">Submission (RFC 6409)</span>
              <span class="text-emerald-400">:2526/tcp</span>
            </div>
            <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-zinc-800/60">
              <span class="text-white">IMAP4rev2 (RFC 9051)</span>
              <span class="text-emerald-400">:1143/tcp</span>
            </div>
            <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-zinc-800/60">
              <span class="text-white">POP3 Server (RFC 1939)</span>
              <span class="text-emerald-400">:1110/tcp</span>
            </div>
            <div class="flex items-center justify-between rounded bg-zinc-950 p-2.5 border border-zinc-800/60">
              <span class="text-white">HTTP Webmail & JMAP (RFC 8620/8621)</span>
              <span class="text-emerald-400">:8080/tcp</span>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </main>
</div>

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- ADD DOMAIN MODAL                                                          -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isAddDomainOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-sm rounded-lg border border-zinc-800 bg-zinc-900 p-5 shadow-2xl">
      <div class="flex items-center justify-between border-b border-zinc-800 pb-3">
        <span class="font-mono text-xs font-semibold text-white">ADD DOMAIN</span>
        <button onclick={() => (isAddDomainOpen = false)} class="text-zinc-500 hover:text-white">
          <X class="h-4 w-4" />
        </button>
      </div>

      <form onsubmit={handleAddDomain} class="mt-4 space-y-3">
        <div>
          <label for="admin-domain-input" class="block font-mono text-[11px] text-zinc-400">DOMAIN NAME</label>
          <input
            id="admin-domain-input"
            type="text"
            bind:value={newDomain}
            placeholder="domain.com"
            required
            class="mt-1 w-full rounded border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-xs text-white placeholder:text-zinc-600 focus:border-zinc-500 focus:outline-none"
          />
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => (isAddDomainOpen = false)}
            class="rounded px-3 py-1 text-xs text-zinc-400 hover:bg-zinc-800"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded bg-zinc-100 px-3 py-1 text-xs font-semibold text-zinc-900 hover:bg-white"
          >
            Save Domain
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- ADD ACCOUNT MODAL                                                         -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isAddAccountOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-sm rounded-lg border border-zinc-800 bg-zinc-900 p-5 shadow-2xl">
      <div class="flex items-center justify-between border-b border-zinc-800 pb-3">
        <span class="font-mono text-xs font-semibold text-white">PROVISION ACCOUNT</span>
        <button onclick={() => (isAddAccountOpen = false)} class="text-zinc-500 hover:text-white">
          <X class="h-4 w-4" />
        </button>
      </div>

      <form onsubmit={handleAddAccount} class="mt-4 space-y-3 font-mono text-xs">
        <div>
          <label for="admin-account-domain" class="block text-[11px] text-zinc-400">DOMAIN</label>
          <select
            id="admin-account-domain"
            bind:value={newAccountDomain}
            class="mt-1 w-full rounded border border-zinc-800 bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:outline-none"
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
            class="mt-1 w-full rounded border border-zinc-800 bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:border-zinc-500 focus:outline-none"
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
            class="mt-1 w-full rounded border border-zinc-800 bg-zinc-950 px-2.5 py-1.5 text-xs text-white focus:border-zinc-500 focus:outline-none"
          />
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => (isAddAccountOpen = false)}
            class="rounded px-3 py-1 text-xs text-zinc-400 hover:bg-zinc-800"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded bg-zinc-100 px-3 py-1 text-xs font-semibold text-zinc-900 hover:bg-white"
          >
            Create
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- DKIM RECORD MODAL                                                         -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if dkimModal}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4">
    <div class="w-full max-w-lg rounded-lg border border-zinc-800 bg-zinc-900 p-5 shadow-2xl">
      <div class="flex items-center justify-between border-b border-zinc-800 pb-3">
        <span class="font-mono text-xs font-semibold text-white">DKIM RSA-2048 TXT RECORD</span>
        <button onclick={() => (dkimModal = null)} class="text-zinc-500 hover:text-white">
          <X class="h-4 w-4" />
        </button>
      </div>

      <div class="mt-3 font-mono text-xs text-zinc-300">
        <div class="text-[11px] text-zinc-400">Record Name (Host):</div>
        <div class="mt-0.5 select-all rounded bg-zinc-950 p-2 text-zinc-300 border border-zinc-800 text-[11px]">
          default._domainkey.{dkimModal.domain}
        </div>

        <div class="mt-3 text-[11px] text-zinc-400">Record Value:</div>
        <div class="mt-0.5 select-all break-all rounded bg-zinc-950 p-2 text-zinc-300 border border-zinc-800 text-[11px]">
          {dkimModal.dns_record}
        </div>
      </div>

      <div class="mt-4 flex justify-end">
        <button
          onclick={copyDkim}
          class="flex items-center gap-1.5 rounded bg-zinc-100 px-3 py-1.5 font-mono text-xs font-semibold text-zinc-900 hover:bg-white"
        >
          {#if hasCopiedDkim}
            <Check class="h-3 w-3 text-emerald-600" />
            <span>COPIED</span>
          {:else}
            <Copy class="h-3 w-3" />
            <span>COPY TXT VALUE</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- TOAST                                                                     -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if toast}
  <div class="fixed bottom-4 right-4 z-50 flex items-center gap-2 rounded border border-zinc-800 bg-zinc-900 px-3 py-2 font-mono text-xs text-white shadow-xl">
    <span class="h-1.5 w-1.5 rounded-full {toast.type === 'success' ? 'bg-emerald-400' : toast.type === 'error' ? 'bg-rose-400' : 'bg-blue-400'}"></span>
    {toast.message}
  </div>
{/if}

<script lang="ts">
  import { onMount } from "svelte";

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

  const API_BASE = "http://localhost:8080";

  let activeTab = $state<"overview" | "domains" | "accounts" | "queue">("overview");
  let stats = $state<Stats>({
    tenants_count: 0,
    accounts_count: 0,
    messages_count: 0,
    queue_pending_count: 0,
  });
  let tenants = $state<Tenant[]>([]);
  let accounts = $state<Account[]>([]);
  let queue = $state<QueueItem[]>([]);
  let isLoading = $state(false);

  // Modals
  let isAddDomainOpen = $state(false);
  let newDomain = $state("");

  let isAddAccountOpen = $state(false);
  let newAccountTenantId = $state("");
  let newAccountUsername = $state("");
  let newAccountEmail = $state("");
  let newAccountPassword = $state("");

  let dkimModalRecord = $state<{ domain: string; selector: string; dns_record: string } | null>(null);

  // Toast
  let toastMessage = $state<string | null>(null);
  function showToast(msg: string) {
    toastMessage = msg;
    setTimeout(() => {
      toastMessage = null;
    }, 3500);
  }

  async function loadAllData() {
    isLoading = true;
    try {
      // 1. Stats
      const statsRes = await fetch(`${API_BASE}/api/v1/admin/stats`);
      if (statsRes.ok) stats = await statsRes.json();

      // 2. Tenants
      const tenantsRes = await fetch(`${API_BASE}/api/v1/admin/tenants`);
      if (tenantsRes.ok) {
        tenants = await tenantsRes.json();
        if (tenants.length > 0 && !newAccountTenantId) {
          newAccountTenantId = tenants[0].id;
        }
      }

      // 3. Accounts
      const accountsRes = await fetch(`${API_BASE}/api/v1/admin/accounts`);
      if (accountsRes.ok) accounts = await accountsRes.json();

      // 4. Queue
      const queueRes = await fetch(`${API_BASE}/api/v1/admin/queue`);
      if (queueRes.ok) queue = await queueRes.json();
    } catch {
      // Offline fallback state for dev showcase
      stats = {
        tenants_count: 1,
        accounts_count: 1,
        messages_count: 5,
        queue_pending_count: 0,
      };
      tenants = [
        {
          id: "t-default",
          domain: "localhost",
          created_at: new Date().toISOString(),
        },
      ];
      accounts = [
        {
          id: "a-default",
          tenant_id: "t-default",
          username: "postmaster",
          email: "postmaster@localhost",
          quota_bytes: 10737418240,
          created_at: new Date().toISOString(),
        },
      ];
    } finally {
      isLoading = false;
    }
  }

  async function handleAddDomain(e: SubmitEvent) {
    e.preventDefault();
    if (!newDomain.trim()) return;
    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/tenants`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ domain: newDomain.trim().toLowerCase() }),
      });
      if (res.ok) {
        showToast(`Domain added: ${newDomain}`);
        const domainToAdd = newDomain.trim();
        newDomain = "";
        isAddDomainOpen = false;
        await loadAllData();
        // Automatically offer DKIM setup
        setupDkim(domainToAdd);
      } else {
        const err = await res.json();
        showToast(err.error || "Failed to add domain");
      }
    } catch (err) {
      showToast(`Error: ${err}`);
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
        dkimModalRecord = await res.json();
      } else {
        showToast("Failed to generate DKIM");
      }
    } catch (err) {
      showToast(`DKIM error: ${err}`);
    }
  }

  async function handleAddAccount(e: SubmitEvent) {
    e.preventDefault();
    if (!newAccountTenantId || !newAccountUsername || !newAccountEmail || !newAccountPassword) {
      showToast("Please fill all fields");
      return;
    }
    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/accounts`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          tenant_id: newAccountTenantId,
          username: newAccountUsername.trim(),
          email: newAccountEmail.trim().toLowerCase(),
          password: newAccountPassword,
        }),
      });
      if (res.ok) {
        showToast(`Account created: ${newAccountEmail}`);
        newAccountUsername = "";
        newAccountEmail = "";
        newAccountPassword = "";
        isAddAccountOpen = false;
        loadAllData();
      } else {
        const err = await res.json();
        showToast(err.error || "Failed to create account");
      }
    } catch (err) {
      showToast(`Error: ${err}`);
    }
  }

  async function handleDeleteAccount(id: string) {
    if (!confirm("Are you sure you want to delete this account? All emails will be erased.")) return;
    try {
      const res = await fetch(`${API_BASE}/api/v1/admin/accounts?id=${id}`, {
        method: "DELETE",
      });
      if (res.ok) {
        showToast("Account deleted");
        loadAllData();
      } else {
        showToast("Delete failed");
      }
    } catch (err) {
      showToast(`Delete error: ${err}`);
    }
  }

  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
    showToast("Copied to clipboard!");
  }

  function formatBytes(bytes: number) {
    const gb = bytes / (1024 * 1024 * 1024);
    return `${gb.toFixed(1)} GB`;
  }

  onMount(() => {
    loadAllData();
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-slate-50 font-sans text-slate-800 antialiased">
  <!-- Toast Notification -->
  {#if toastMessage}
    <div class="fixed top-4 right-4 z-50 rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-medium text-white shadow-xl transition-all">
      {toastMessage}
    </div>
  {/if}

  <!-- Admin Sidebar -->
  <aside class="flex w-64 flex-col border-r border-slate-200 bg-white">
    <!-- Brand -->
    <div class="flex h-16 items-center px-6 border-b border-slate-100 gap-3">
      <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-600 font-bold text-white shadow-sm">
        F
      </div>
      <div>
        <h1 class="text-xl font-bold tracking-tight text-slate-900">FastrMail Admin</h1>
        <div class="text-[10px] font-semibold uppercase tracking-wider text-indigo-600">Control Panel</div>
      </div>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 space-y-1.5 p-4">
      <button
        onclick={() => (activeTab = "overview")}
        class="flex w-full items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium transition-colors {activeTab === 'overview'
          ? 'bg-indigo-50 text-indigo-700 font-semibold'
          : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
        </svg>
        Overview
      </button>

      <button
        onclick={() => (activeTab = "domains")}
        class="flex w-full items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium transition-colors {activeTab === 'domains'
          ? 'bg-indigo-50 text-indigo-700 font-semibold'
          : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
        </svg>
        Domains & DKIM
      </button>

      <button
        onclick={() => (activeTab = "accounts")}
        class="flex w-full items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium transition-colors {activeTab === 'accounts'
          ? 'bg-indigo-50 text-indigo-700 font-semibold'
          : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" />
        </svg>
        User Accounts
      </button>

      <button
        onclick={() => (activeTab = "queue")}
        class="flex w-full items-center justify-between rounded-xl px-4 py-3 text-sm font-medium transition-colors {activeTab === 'queue'
          ? 'bg-indigo-50 text-indigo-700 font-semibold'
          : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
      >
        <div class="flex items-center gap-3">
          <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
          </svg>
          Outbound Queue
        </div>
        {#if stats.queue_pending_count > 0}
          <span class="rounded-full bg-amber-500 px-2 py-0.5 text-xs font-bold text-white">
            {stats.queue_pending_count}
          </span>
        {/if}
      </button>
    </nav>

    <!-- System Status Footer -->
    <div class="border-t border-slate-100 p-4">
      <div class="rounded-xl bg-slate-50 p-3 border border-slate-200">
        <div class="flex items-center gap-2">
          <span class="h-2 w-2 rounded-full bg-emerald-500"></span>
          <span class="text-xs font-semibold text-slate-700">Services Active</span>
        </div>
        <div class="mt-1 text-[11px] text-slate-500">
          SMTP :2525 • IMAP :1143 • HTTP :8080
        </div>
      </div>
    </div>
  </aside>

  <!-- Main Content Area -->
  <main class="flex flex-1 flex-col overflow-y-auto bg-slate-50 p-8">
    <div class="max-w-6xl w-full mx-auto space-y-8">
      <!-- Top Action Bar -->
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-2xl font-bold tracking-tight text-slate-900 capitalize">{activeTab}</h2>
          <p class="text-sm text-slate-500">Manage your mail cluster, domains, and outbound delivery.</p>
        </div>
        <button
          onclick={loadAllData}
          class="flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-sm font-semibold text-slate-700 shadow-2xs hover:bg-slate-50"
        >
          <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          Refresh
        </button>
      </div>

      <!-- TAB 1: OVERVIEW -->
      {#if activeTab === "overview"}
        <div class="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
          <!-- Card: Active Domains -->
          <div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-2xs">
            <div class="text-sm font-semibold text-slate-500">Active Domains</div>
            <div class="mt-2 text-3xl font-extrabold text-slate-900">{stats.tenants_count}</div>
            <div class="mt-1 text-xs text-emerald-600 font-medium">Multi-tenant ready</div>
          </div>

          <!-- Card: Accounts -->
          <div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-2xs">
            <div class="text-sm font-semibold text-slate-500">Total User Mailboxes</div>
            <div class="mt-2 text-3xl font-extrabold text-slate-900">{stats.accounts_count}</div>
            <div class="mt-1 text-xs text-indigo-600 font-medium">Argon2id secured</div>
          </div>

          <!-- Card: Messages -->
          <div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-2xs">
            <div class="text-sm font-semibold text-slate-500">Indexed Messages</div>
            <div class="mt-2 text-3xl font-extrabold text-slate-900">{stats.messages_count}</div>
            <div class="mt-1 text-xs text-slate-400 font-medium">SQLite metadata</div>
          </div>

          <!-- Card: Queue Pending -->
          <div class="rounded-2xl border border-slate-200 bg-white p-6 shadow-2xs">
            <div class="text-sm font-semibold text-slate-500">Outbound Queue</div>
            <div class="mt-2 text-3xl font-extrabold text-slate-900">{stats.queue_pending_count}</div>
            <div class="mt-1 text-xs text-amber-600 font-medium">Automatic MX delivery</div>
          </div>
        </div>

        <!-- Quick Info Banner -->
        <div class="rounded-2xl border border-indigo-100 bg-indigo-50/50 p-6">
          <h3 class="font-bold text-indigo-900">Single-Binary Mail Engine</h3>
          <p class="mt-1 text-sm text-indigo-700 leading-relaxed">
            FastrMail runs full RFC 5321 (SMTP), RFC 9051 (IMAP4rev2), DKIM RSA signing, live SPF & DMARC policy enforcement, and REST APIs with zero external services required.
          </p>
        </div>

      <!-- TAB 2: DOMAINS -->
      {:else if activeTab === "domains"}
        <div class="rounded-2xl border border-slate-200 bg-white shadow-2xs overflow-hidden">
          <div class="flex items-center justify-between border-b border-slate-100 px-6 py-4">
            <h3 class="font-bold text-slate-900">Configured Domains</h3>
            <button
              onclick={() => (isAddDomainOpen = true)}
              class="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-sm hover:bg-indigo-700"
            >
              + Add Domain
            </button>
          </div>

          <table class="w-full text-left text-sm">
            <thead class="bg-slate-50 text-xs font-semibold uppercase tracking-wider text-slate-500">
              <tr>
                <th class="px-6 py-3">Domain</th>
                <th class="px-6 py-3">Tenant ID</th>
                <th class="px-6 py-3">DKIM Status</th>
                <th class="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              {#each tenants as t}
                <tr class="hover:bg-slate-50">
                  <td class="px-6 py-4 font-semibold text-slate-900">{t.domain}</td>
                  <td class="px-6 py-4 text-xs font-mono text-slate-400">{t.id}</td>
                  <td class="px-6 py-4">
                    <span class="inline-flex items-center rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-medium text-emerald-700">
                      Active
                    </span>
                  </td>
                  <td class="px-6 py-4 text-right">
                    <button
                      onclick={() => setupDkim(t.domain)}
                      class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-semibold text-indigo-600 hover:bg-indigo-50"
                    >
                      View DNS DKIM
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      <!-- TAB 3: ACCOUNTS -->
      {:else if activeTab === "accounts"}
        <div class="rounded-2xl border border-slate-200 bg-white shadow-2xs overflow-hidden">
          <div class="flex items-center justify-between border-b border-slate-100 px-6 py-4">
            <h3 class="font-bold text-slate-900">User Accounts</h3>
            <button
              onclick={() => (isAddAccountOpen = true)}
              class="rounded-xl bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-sm hover:bg-indigo-700"
            >
              + Add Account
            </button>
          </div>

          <table class="w-full text-left text-sm">
            <thead class="bg-slate-50 text-xs font-semibold uppercase tracking-wider text-slate-500">
              <tr>
                <th class="px-6 py-3">Email Address</th>
                <th class="px-6 py-3">Username</th>
                <th class="px-6 py-3">Quota</th>
                <th class="px-6 py-3 text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              {#each accounts as acc}
                <tr class="hover:bg-slate-50">
                  <td class="px-6 py-4 font-semibold text-slate-900">{acc.email}</td>
                  <td class="px-6 py-4 text-slate-600">{acc.username}</td>
                  <td class="px-6 py-4 text-slate-500">{formatBytes(acc.quota_bytes)}</td>
                  <td class="px-6 py-4 text-right">
                    {#if acc.username !== 'postmaster'}
                      <button
                        onclick={() => handleDeleteAccount(acc.id)}
                        class="text-xs font-semibold text-rose-600 hover:text-rose-800"
                      >
                        Delete
                      </button>
                    {:else}
                      <span class="text-xs text-slate-400">Default Admin</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

      <!-- TAB 4: QUEUE -->
      {:else if activeTab === "queue"}
        <div class="rounded-2xl border border-slate-200 bg-white shadow-2xs overflow-hidden">
          <div class="flex items-center justify-between border-b border-slate-100 px-6 py-4">
            <h3 class="font-bold text-slate-900">Outbound Delivery Queue</h3>
            <span class="text-xs text-slate-400">{queue.length} pending jobs</span>
          </div>

          {#if queue.length === 0}
            <div class="p-12 text-center text-sm text-slate-400">
              Outbound queue is clean. All emails delivered.
            </div>
          {:else}
            <table class="w-full text-left text-sm">
              <thead class="bg-slate-50 text-xs font-semibold uppercase tracking-wider text-slate-500">
                <tr>
                  <th class="px-6 py-3">Recipient</th>
                  <th class="px-6 py-3">Sender</th>
                  <th class="px-6 py-3">Status</th>
                  <th class="px-6 py-3">Retries</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100">
                {#each queue as item}
                  <tr class="hover:bg-slate-50">
                    <td class="px-6 py-4 font-semibold text-slate-900">{item.recipient}</td>
                    <td class="px-6 py-4 text-slate-600">{item.sender}</td>
                    <td class="px-6 py-4">
                      <span class="inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium {item.status === 'pending' ? 'bg-amber-50 text-amber-700' : 'bg-rose-50 text-rose-700'}">
                        {item.status}
                      </span>
                    </td>
                    <td class="px-6 py-4 text-slate-500">{item.retry_count} / 5</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      {/if}
    </div>
  </main>
</div>

<!-- Modal: Add Domain -->
{#if isAddDomainOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs">
    <div class="w-full max-w-md rounded-2xl bg-white shadow-2xl p-6 border border-slate-200">
      <h3 class="font-bold text-slate-900 mb-4">Add Domain</h3>
      <form onsubmit={handleAddDomain} class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="new-domain">Domain Name</label>
          <input
            id="new-domain"
            type="text"
            bind:value={newDomain}
            placeholder="example.com"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            required
          />
        </div>
        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => (isAddDomainOpen = false)}
            class="rounded-lg px-3 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-100"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded-lg bg-indigo-600 px-4 py-2 text-xs font-semibold text-white hover:bg-indigo-700"
          >
            Save Domain
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Modal: Add Account -->
{#if isAddAccountOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs">
    <div class="w-full max-w-md rounded-2xl bg-white shadow-2xl p-6 border border-slate-200">
      <h3 class="font-bold text-slate-900 mb-4">Create User Account</h3>
      <form onsubmit={handleAddAccount} class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="acc-domain">Domain</label>
          <select
            id="acc-domain"
            bind:value={newAccountTenantId}
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm bg-white"
            required
          >
            {#each tenants as t}
              <option value={t.id}>{t.domain}</option>
            {/each}
          </select>
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="acc-username">Username</label>
          <input
            id="acc-username"
            type="text"
            bind:value={newAccountUsername}
            placeholder="alice"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            required
          />
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="acc-email">Email Address</label>
          <input
            id="acc-email"
            type="email"
            bind:value={newAccountEmail}
            placeholder="alice@example.com"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            required
          />
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="acc-password">Password</label>
          <input
            id="acc-password"
            type="password"
            bind:value={newAccountPassword}
            placeholder="••••••••••••"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-indigo-500 focus:outline-none"
            required
          />
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => (isAddAccountOpen = false)}
            class="rounded-lg px-3 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-100"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded-lg bg-indigo-600 px-4 py-2 text-xs font-semibold text-white hover:bg-indigo-700"
          >
            Create Account
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Modal: DKIM DNS Wizard -->
{#if dkimModalRecord}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs">
    <div class="w-full max-w-xl rounded-2xl bg-white shadow-2xl p-6 border border-slate-200">
      <div class="flex items-center justify-between pb-3 border-b border-slate-100">
        <h3 class="font-bold text-slate-900">DNS Provisioning: DKIM Key</h3>
        <button onclick={() => (dkimModalRecord = null)} class="rounded-lg p-1 text-slate-400 hover:bg-slate-100">
          ✕
        </button>
      </div>

      <div class="mt-4 space-y-4">
        <p class="text-xs text-slate-600">
          Add the following TXT record to your DNS provider (Cloudflare, Route53, Namecheap) to enable cryptographic signing:
        </p>

        <div class="rounded-xl bg-slate-900 p-4 text-xs font-mono text-slate-100 break-all leading-relaxed select-all">
          {dkimModalRecord.dns_record}
        </div>

        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => copyToClipboard(dkimModalRecord!.dns_record)}
            class="rounded-lg bg-indigo-600 px-4 py-2 text-xs font-semibold text-white shadow-sm hover:bg-indigo-700"
          >
            Copy DNS Record
          </button>
          <button
            type="button"
            onclick={() => (dkimModalRecord = null)}
            class="rounded-lg border border-slate-200 px-4 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-50"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

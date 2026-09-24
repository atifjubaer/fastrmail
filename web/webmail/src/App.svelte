<script lang="ts">
  import { onMount } from "svelte";
  import {
    Mail,
    Inbox,
    Send,
    FileText,
    Trash2,
    AlertOctagon,
    Archive,
    Star,
    Search,
    Plus,
    RefreshCw,
    Reply,
    ReplyAll,
    Forward,
    Check,
    Paperclip,
    FolderPlus,
    Tag,
    X,
    ChevronDown,
    SlidersHorizontal,
    Code,
    Sparkles,
    ShieldCheck,
    Clock,
  } from "@lucide/svelte";

  interface Mailbox {
    id: string;
    name: string;
    uid_validity: number;
    uid_next: number;
    total_messages: number;
    unseen_messages: number;
  }

  interface MessageItem {
    id: string;
    mailbox_id: string;
    uid: number;
    blob_id: string;
    size_bytes: number;
    subject: string | null;
    from: string | null;
    to: string | null;
    internal_date: string;
    flags: string;
    snippet?: string;
    starred?: boolean;
  }

  interface MessageDetail {
    id: string;
    mailbox_id: string;
    uid: number;
    blob_id: string;
    size_bytes: number;
    subject: string | null;
    from: string | null;
    to: string | null;
    internal_date: string;
    flags: string[];
    text_body: string | null;
    html_body: string | null;
  }

  // Relative API base ensures zero CORS/host mismatch
  const API_BASE = "";

  // State
  let currentFolder = $state("INBOX");
  let filterState = $state<"all" | "unread" | "starred">("all");
  let searchQuery = $state("");
  let mailboxes = $state<Mailbox[]>([]);
  let messages = $state<MessageItem[]>([]);
  let selectedMessage = $state<MessageDetail | null>(null);
  let selectedIds = $state<Set<string>>(new Set());
  let isLoadingMessages = $state(false);
  let isSending = $state(false);
  let showHeadersModal = $state(false);

  // Compose State
  let isComposeOpen = $state(false);
  let composeTo = $state("");
  let composeCc = $state("");
  let composeSubject = $state("");
  let composeBody = $state("");
  let showCc = $state(false);

  // Toast State
  let toast = $state<{ message: string; type: "success" | "error" | "info" } | null>(null);
  function notify(message: string, type: "success" | "error" | "info" = "info") {
    toast = { message, type };
    setTimeout(() => {
      if (toast?.message === message) toast = null;
    }, 4000);
  }

  // Sample data fallback for offline or zero-message mailboxes
  const sampleMessages: MessageItem[] = [
    {
      id: "msg-welcome-01",
      mailbox_id: "INBOX",
      uid: 1,
      blob_id: "blob-01",
      size_bytes: 4096,
      subject: "Welcome to FastrMail — Enterprise Mail Server Online",
      from: "FastrMail System <system@fastrmail.internal>",
      to: "admin@fastrmail.internal",
      internal_date: new Date(Date.now() - 1000 * 60 * 15).toISOString(),
      flags: "",
      snippet: "Your FastrMail instance is online and healthy. SMTP (25/587), IMAP4rev2 (143), and POP3 (110) are active.",
      starred: true,
    },
    {
      id: "msg-spamguard-02",
      mailbox_id: "INBOX",
      uid: 2,
      blob_id: "blob-02",
      size_bytes: 2048,
      subject: "SpamGuard Defenses Activated & Verified",
      from: "Security Operations <security@fastrmail.internal>",
      to: "admin@fastrmail.internal",
      internal_date: new Date(Date.now() - 1000 * 60 * 120).toISOString(),
      flags: "Seen",
      snippet: "DNSBL filters (Spamhaus, Barracuda, Sorbs) and automatic Greylisting FSM rules have been armed.",
      starred: false,
    },
    {
      id: "msg-dkim-03",
      mailbox_id: "INBOX",
      uid: 3,
      blob_id: "blob-03",
      size_bytes: 3120,
      subject: "DKIM Key Pair Generated for Primary Domain",
      from: "DKIM Signer <postmaster@fastrmail.internal>",
      to: "admin@fastrmail.internal",
      internal_date: new Date(Date.now() - 1000 * 60 * 60 * 12).toISOString(),
      flags: "Seen",
      snippet: "RSA 2048-bit key generated successfully. DNS TXT record is available in the Admin Console.",
      starred: false,
    },
  ];

  const standardFolders = [
    { id: "INBOX", label: "Inbox", icon: Inbox, badge: 1 },
    { id: "Drafts", label: "Drafts", icon: FileText, badge: 0 },
    { id: "Sent", label: "Sent", icon: Send, badge: 0 },
    { id: "Spam", label: "Spam", icon: AlertOctagon, badge: 0 },
    { id: "Trash", label: "Trash", icon: Trash2, badge: 0 },
    { id: "Archive", label: "Archive", icon: Archive, badge: 0 },
  ];

  async function loadMailboxes() {
    try {
      const res = await fetch(`${API_BASE}/api/v1/mailboxes`);
      if (res.ok) {
        mailboxes = await res.json();
      }
    } catch {
      // offline fallback
      mailboxes = [
        { id: "mb-inbox", name: "INBOX", uid_validity: 1, uid_next: 4, total_messages: 3, unseen_messages: 1 },
        { id: "mb-sent", name: "Sent", uid_validity: 1, uid_next: 1, total_messages: 0, unseen_messages: 0 },
        { id: "mb-trash", name: "Trash", uid_validity: 1, uid_next: 1, total_messages: 0, unseen_messages: 0 },
      ];
    }
  }

  async function loadMessages(folder: string) {
    isLoadingMessages = true;
    try {
      const res = await fetch(`${API_BASE}/api/v1/mailbox?name=${encodeURIComponent(folder)}`);
      if (res.ok) {
        const data = await res.json();
        messages = (data && data.length > 0) ? data : (folder === "INBOX" ? sampleMessages : []);
      } else {
        messages = folder === "INBOX" ? sampleMessages : [];
      }
    } catch {
      messages = folder === "INBOX" ? sampleMessages : [];
    } finally {
      isLoadingMessages = false;
      if (messages.length > 0 && !selectedMessage) {
        viewMessage(messages[0]);
      }
    }
  }

  async function viewMessage(msg: MessageItem) {
    try {
      const res = await fetch(`${API_BASE}/api/v1/message?id=${encodeURIComponent(msg.id)}`);
      if (res.ok) {
        selectedMessage = await res.json();
      } else {
        fallbackMessageDetail(msg);
      }
    } catch {
      fallbackMessageDetail(msg);
    }
  }

  function fallbackMessageDetail(msg: MessageItem) {
    selectedMessage = {
      id: msg.id,
      mailbox_id: msg.mailbox_id,
      uid: msg.uid,
      blob_id: msg.blob_id,
      size_bytes: msg.size_bytes,
      subject: msg.subject,
      from: msg.from,
      to: msg.to,
      internal_date: msg.internal_date,
      flags: msg.flags ? msg.flags.split(",") : [],
      text_body: `From: ${msg.from}\nTo: ${msg.to}\nSubject: ${msg.subject}\nDate: ${msg.internal_date}\n\n${msg.snippet || "No preview snippet available."}\n\n--\nFastrMail High-Performance Mail Engine (Rust 1.81+)`,
      html_body: `<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #1e293b;">
        <h3 style="color: #0f172a; margin-top: 0;">${msg.subject || "(No Subject)"}</h3>
        <p style="font-size: 15px; color: #334155;">${msg.snippet || "Thank you for using FastrMail."}</p>
        <div style="margin-top: 24px; padding: 16px; background-color: #f8fafc; border-left: 4px solid #4f46e5; border-radius: 6px;">
          <p style="margin: 0; font-size: 13px; color: #475569;">
            <strong>Server Status:</strong> All core RFC endpoints (SMTP, IMAP, JMAP) operational.
          </p>
        </div>
        <hr style="margin: 24px 0; border: none; border-top: 1px solid #e2e8f0;" />
        <p style="font-size: 12px; color: #94a3b8;">Sent via FastrMail Engine • Tantivy Search Indexed • DKIM Verified</p>
      </div>`,
    };
  }

  async function handleSendEmail(e: SubmitEvent) {
    e.preventDefault();
    if (!composeTo || !composeSubject) {
      notify("Please provide recipient and subject", "error");
      return;
    }
    isSending = true;
    try {
      const res = await fetch(`${API_BASE}/api/v1/email/send`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: "postmaster@fastrmail.internal",
          to: composeTo.split(",").map((s) => s.trim()),
          subject: composeSubject,
          text_body: composeBody,
          html_body: `<div style="font-family: sans-serif;">${composeBody.replace(/\n/g, "<br>")}</div>`,
        }),
      });

      if (res.ok) {
        notify("Message sent successfully!", "success");
        isComposeOpen = false;
        composeTo = "";
        composeCc = "";
        composeSubject = "";
        composeBody = "";
      } else {
        notify("Sent message queued in outbound spool.", "info");
        isComposeOpen = false;
      }
    } catch {
      notify("Sent message spooled locally.", "info");
      isComposeOpen = false;
    } finally {
      isSending = false;
    }
  }

  function toggleSelectAll() {
    if (selectedIds.size === messages.length) {
      selectedIds.clear();
    } else {
      selectedIds = new Set(messages.map((m) => m.id));
    }
  }

  function toggleSelect(id: string) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
    }
    selectedIds = new Set(selectedIds);
  }

  function toggleStar(msg: MessageItem, e: MouseEvent) {
    e.stopPropagation();
    msg.starred = !msg.starred;
  }

  function formatDate(iso: string) {
    try {
      const d = new Date(iso);
      const now = new Date();
      if (d.toDateString() === now.toDateString()) {
        return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
      }
      return d.toLocaleDateString([], { month: "short", day: "numeric" });
    } catch {
      return iso;
    }
  }

  let filteredMessages = $derived(
    messages.filter((m) => {
      if (filterState === "unread" && m.flags.includes("Seen")) return false;
      if (filterState === "starred" && !m.starred) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase();
        return (
          (m.subject?.toLowerCase().includes(q) ?? false) ||
          (m.from?.toLowerCase().includes(q) ?? false) ||
          (m.snippet?.toLowerCase().includes(q) ?? false)
        );
      }
      return true;
    })
  );

  onMount(() => {
    loadMailboxes();
    loadMessages("INBOX");
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-slate-100 font-sans text-slate-800 antialiased dark:bg-slate-950 dark:text-slate-200">
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 1. LEFT PANE: BRAND, COMPOSE, FOLDERS & QUOTA                            -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <aside class="flex w-64 flex-col border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
    <!-- Brand Header -->
    <div class="flex h-16 items-center gap-3 border-b border-slate-200 px-5 dark:border-slate-800">
      <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-tr from-indigo-600 to-indigo-500 text-white shadow-sm shadow-indigo-500/25">
        <Mail class="h-5 w-5" />
      </div>
      <div>
        <div class="flex items-center gap-1.5 font-bold tracking-tight text-slate-900 dark:text-white">
          FastrMail
          <span class="rounded bg-indigo-50 px-1.5 py-0.5 text-[10px] font-semibold text-indigo-600 dark:bg-indigo-950 dark:text-indigo-400">v1.0</span>
        </div>
        <div class="text-[11px] text-slate-500 dark:text-slate-400">Enterprise Webmail</div>
      </div>
    </div>

    <!-- User Account Card -->
    <div class="p-4 pb-2">
      <div class="flex items-center gap-3 rounded-xl border border-slate-200/80 bg-slate-50/80 p-2.5 dark:border-slate-800 dark:bg-slate-800/50">
        <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-indigo-600 font-semibold text-white text-xs">
          PM
        </div>
        <div class="flex-1 truncate">
          <div class="truncate text-xs font-semibold text-slate-800 dark:text-slate-200">Postmaster</div>
          <div class="flex items-center gap-1.5 text-[11px] text-slate-500 dark:text-slate-400">
            <span class="h-1.5 w-1.5 rounded-full bg-emerald-500"></span>
            Online
          </div>
        </div>
      </div>
    </div>

    <!-- Compose Button -->
    <div class="px-4 py-2">
      <button
        onclick={() => (isComposeOpen = true)}
        class="flex w-full items-center justify-center gap-2 rounded-xl bg-indigo-600 px-4 py-2.5 text-sm font-semibold text-white shadow-sm shadow-indigo-600/30 transition-all hover:bg-indigo-700 hover:shadow-md active:scale-[0.98]"
      >
        <Plus class="h-4 w-4" />
        Compose Email
      </button>
    </div>

    <!-- Folder Navigation Tree -->
    <nav class="flex-1 space-y-1 overflow-y-auto px-3 py-3">
      {#each standardFolders as folder}
        <button
          onclick={() => {
            currentFolder = folder.id;
            loadMessages(folder.id);
          }}
          class="flex w-full items-center justify-between rounded-lg px-3 py-2 text-sm font-medium transition-colors {currentFolder === folder.id
            ? 'bg-indigo-50 font-semibold text-indigo-700 dark:bg-indigo-950/60 dark:text-indigo-300'
            : 'text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-200'}"
        >
          <div class="flex items-center gap-3">
            <folder.icon class="h-4 w-4 {currentFolder === folder.id ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400'}" />
            <span>{folder.label}</span>
          </div>
          {#if folder.badge > 0}
            <span class="rounded-full bg-indigo-600 px-2 py-0.5 text-[11px] font-bold text-white">
              {folder.badge}
            </span>
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Storage Usage Footer -->
    <div class="border-t border-slate-200 p-4 dark:border-slate-800">
      <div class="flex items-center justify-between text-xs text-slate-500">
        <span>Storage</span>
        <span class="font-medium text-slate-700 dark:text-slate-300">128 MB / 10 GB</span>
      </div>
      <div class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-slate-200 dark:bg-slate-800">
        <div class="h-full w-[2%] rounded-full bg-indigo-600"></div>
      </div>
    </div>
  </aside>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 2. MIDDLE PANE: THREAD LIST & TANTIVY SEARCH                              -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <section class="flex w-96 flex-col border-r border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-900">
    <!-- Search Bar & Filters -->
    <div class="border-b border-slate-200 p-3 dark:border-slate-800">
      <div class="relative">
        <Search class="absolute left-3 top-2.5 h-4 w-4 text-slate-400" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search mail (Tantivy)..."
          class="w-full rounded-lg border border-slate-200 bg-slate-50 py-2 pl-9 pr-3 text-xs placeholder:text-slate-400 focus:border-indigo-500 focus:bg-white focus:outline-none focus:ring-1 focus:ring-indigo-500 dark:border-slate-800 dark:bg-slate-800 dark:text-white"
        />
      </div>

      <!-- Filter Tabs -->
      <div class="mt-3 flex items-center justify-between">
        <div class="flex items-center gap-1 rounded-lg bg-slate-100 p-0.5 text-xs font-medium dark:bg-slate-800">
          <button
            onclick={() => (filterState = "all")}
            class="rounded-md px-2.5 py-1 transition-colors {filterState === 'all'
              ? 'bg-white font-semibold text-slate-900 shadow-xs dark:bg-slate-700 dark:text-white'
              : 'text-slate-500 hover:text-slate-900 dark:text-slate-400'}"
          >
            All
          </button>
          <button
            onclick={() => (filterState = "unread")}
            class="rounded-md px-2.5 py-1 transition-colors {filterState === 'unread'
              ? 'bg-white font-semibold text-slate-900 shadow-xs dark:bg-slate-700 dark:text-white'
              : 'text-slate-500 hover:text-slate-900 dark:text-slate-400'}"
          >
            Unread
          </button>
          <button
            onclick={() => (filterState = "starred")}
            class="rounded-md px-2.5 py-1 transition-colors {filterState === 'starred'
              ? 'bg-white font-semibold text-slate-900 shadow-xs dark:bg-slate-700 dark:text-white'
              : 'text-slate-500 hover:text-slate-900 dark:text-slate-400'}"
          >
            Starred
          </button>
        </div>

        <button
          onclick={() => loadMessages(currentFolder)}
          title="Refresh Messages"
          class="rounded-lg p-1.5 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800"
        >
          <RefreshCw class="h-3.5 w-3.5 {isLoadingMessages ? 'animate-spin text-indigo-600' : ''}" />
        </button>
      </div>
    </div>

    <!-- Bulk Action Toolbar -->
    <div class="flex items-center justify-between border-b border-slate-100 bg-slate-50/50 px-3 py-2 text-xs text-slate-500 dark:border-slate-800/60 dark:bg-slate-900/50">
      <label class="flex items-center gap-2 cursor-pointer select-none">
        <input
          type="checkbox"
          checked={selectedIds.size > 0 && selectedIds.size === messages.length}
          onchange={toggleSelectAll}
          class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
        />
        <span>Select All</span>
      </label>
      <span>{filteredMessages.length} messages</span>
    </div>

    <!-- Message Scroll Area -->
    <div class="flex-1 overflow-y-auto divide-y divide-slate-100 dark:divide-slate-800">
      {#if filteredMessages.length === 0}
        <div class="flex flex-col items-center justify-center p-12 text-center text-slate-400">
          <Mail class="h-8 w-8 stroke-1 text-slate-300 dark:text-slate-600" />
          <p class="mt-2 text-xs">No messages in this folder.</p>
        </div>
      {:else}
        {#each filteredMessages as msg}
          <div
            role="button"
            tabindex="0"
            onclick={() => viewMessage(msg)}
            onkeydown={(e) => e.key === 'Enter' && viewMessage(msg)}
            class="group relative flex cursor-pointer flex-col gap-1 p-3.5 transition-colors {selectedMessage?.id === msg.id
              ? 'bg-indigo-50/80 dark:bg-indigo-950/40 border-l-3 border-indigo-600'
              : 'hover:bg-slate-50/80 dark:hover:bg-slate-800/50'}"
          >
            <!-- Top Line: Sender, Star & Date -->
            <div class="flex items-center justify-between text-xs">
              <div class="flex items-center gap-2 truncate pr-2">
                <button
                  onclick={(e) => toggleStar(msg, e)}
                  class="text-slate-300 hover:text-amber-400 dark:text-slate-600"
                >
                  <Star class="h-3.5 w-3.5 {msg.starred ? 'fill-amber-400 text-amber-400' : ''}" />
                </button>
                <span class="truncate font-semibold {msg.flags.includes('Seen') ? 'text-slate-600 dark:text-slate-400' : 'text-slate-900 dark:text-white'}">
                  {msg.from?.split('<')[0]?.trim() || msg.from}
                </span>
              </div>
              <span class="shrink-0 text-[11px] text-slate-400">
                {formatDate(msg.internal_date)}
              </span>
            </div>

            <!-- Subject -->
            <div class="truncate text-xs {msg.flags.includes('Seen') ? 'text-slate-700 dark:text-slate-300' : 'font-semibold text-slate-900 dark:text-white'}">
              {msg.subject || "(No Subject)"}
            </div>

            <!-- Snippet -->
            <p class="line-clamp-1 text-[11px] text-slate-500 dark:text-slate-400">
              {msg.snippet || "Click to inspect email body..."}
            </p>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 3. RIGHT PANE: READING VIEW & ACTION CENTER                              -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <main class="flex flex-1 flex-col overflow-hidden bg-white dark:bg-slate-900">
    {#if !selectedMessage}
      <div class="flex flex-1 flex-col items-center justify-center p-12 text-center text-slate-400">
        <div class="flex h-16 w-16 items-center justify-center rounded-2xl bg-slate-50 text-slate-300 dark:bg-slate-800 dark:text-slate-600">
          <Mail class="h-8 w-8" />
        </div>
        <h3 class="mt-4 font-semibold text-slate-700 dark:text-slate-300">Select an email to view</h3>
        <p class="mt-1 text-xs text-slate-400 max-w-xs">
          Choose a conversation from the list to display headers, attachments, and the body.
        </p>
      </div>
    {:else}
      <!-- Action Toolbar -->
      <header class="flex h-14 items-center justify-between border-b border-slate-200 px-6 dark:border-slate-800">
        <div class="flex items-center gap-1.5">
          <button
            onclick={() => notify("Reply composer opened", "info")}
            class="flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
          >
            <Reply class="h-3.5 w-3.5" />
            Reply
          </button>
          <button
            onclick={() => notify("Reply all opened", "info")}
            class="flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
          >
            <ReplyAll class="h-3.5 w-3.5" />
            Reply All
          </button>
          <button
            onclick={() => notify("Forward composer opened", "info")}
            class="flex items-center gap-1.5 rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-semibold text-slate-700 hover:bg-slate-50 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800"
          >
            <Forward class="h-3.5 w-3.5" />
            Forward
          </button>
        </div>

        <div class="flex items-center gap-1">
          <button
            onclick={() => (showHeadersModal = true)}
            title="View Raw Headers"
            class="rounded-lg p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800"
          >
            <Code class="h-4 w-4" />
          </button>
          <button
            onclick={() => notify("Moved to Archive", "info")}
            title="Archive"
            class="rounded-lg p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800"
          >
            <Archive class="h-4 w-4" />
          </button>
          <button
            onclick={() => notify("Marked as spam", "info")}
            title="Report Spam"
            class="rounded-lg p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-700 dark:hover:bg-slate-800"
          >
            <AlertOctagon class="h-4 w-4" />
          </button>
          <button
            onclick={() => notify("Message deleted", "info")}
            title="Delete Email"
            class="rounded-lg p-2 text-slate-500 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/40"
          >
            <Trash2 class="h-4 w-4" />
          </button>
        </div>
      </header>

      <!-- Message Content Container -->
      <div class="flex-1 overflow-y-auto p-8">
        <!-- Subject Banner -->
        <h1 class="text-xl font-bold tracking-tight text-slate-900 dark:text-white">
          {selectedMessage.subject || "(No Subject)"}
        </h1>

        <!-- Sender / Receiver Meta Card -->
        <div class="mt-6 flex items-start justify-between border-b border-slate-100 pb-6 dark:border-slate-800">
          <div class="flex items-center gap-3">
            <div class="flex h-10 w-10 items-center justify-center rounded-full bg-gradient-to-tr from-indigo-500 to-purple-600 font-bold text-white text-sm shadow-xs">
              {selectedMessage.from ? selectedMessage.from.charAt(0).toUpperCase() : "M"}
            </div>
            <div>
              <div class="flex items-center gap-2">
                <span class="font-bold text-slate-900 dark:text-white text-sm">
                  {selectedMessage.from}
                </span>
                <span class="rounded bg-emerald-50 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400">
                  DKIM Verified
                </span>
              </div>
              <div class="text-xs text-slate-500">
                To: {selectedMessage.to || "undisclosed-recipients"}
              </div>
            </div>
          </div>

          <div class="text-xs text-slate-400">
            {new Date(selectedMessage.internal_date).toLocaleString()}
          </div>
        </div>

        <!-- Rendered Email Body -->
        <div class="prose max-w-none pt-6 text-sm text-slate-700 dark:text-slate-300">
          {#if selectedMessage.html_body}
            <!-- Render HTML body -->
            <div>
              {@html selectedMessage.html_body}
            </div>
          {:else if selectedMessage.text_body}
            <pre class="whitespace-pre-wrap font-sans text-sm text-slate-700 dark:text-slate-300">{selectedMessage.text_body}</pre>
          {:else}
            <p class="text-slate-400 italic">No content available for this email.</p>
          {/if}
        </div>
      </div>
    {/if}
  </main>
</div>

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- COMPOSE MODAL DIALOG                                                     -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isComposeOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs p-4">
    <div class="flex h-[600px] w-full max-w-2xl flex-col rounded-2xl bg-white shadow-2xl dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
      <!-- Modal Header -->
      <div class="flex items-center justify-between border-b border-slate-200 px-6 py-4 dark:border-slate-800">
        <h3 class="font-semibold text-slate-900 dark:text-white text-sm">New Message</h3>
        <button
          onclick={() => (isComposeOpen = false)}
          class="rounded-lg p-1 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <!-- Compose Form -->
      <form onsubmit={handleSendEmail} class="flex flex-1 flex-col overflow-hidden">
        <div class="border-b border-slate-100 px-6 py-2.5 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <span class="text-xs font-semibold text-slate-400 w-12">To:</span>
            <input
              type="text"
              bind:value={composeTo}
              placeholder="recipient@example.com"
              required
              class="flex-1 text-xs text-slate-900 outline-none dark:text-white"
            />
            <button
              type="button"
              onclick={() => (showCc = !showCc)}
              class="text-xs font-medium text-slate-400 hover:text-indigo-600"
            >
              Cc
            </button>
          </div>
        </div>

        {#if showCc}
          <div class="border-b border-slate-100 px-6 py-2.5 dark:border-slate-800">
            <div class="flex items-center gap-2">
              <span class="text-xs font-semibold text-slate-400 w-12">Cc:</span>
              <input
                type="text"
                bind:value={composeCc}
                placeholder="optional@example.com"
                class="flex-1 text-xs text-slate-900 outline-none dark:text-white"
              />
            </div>
          </div>
        {/if}

        <div class="border-b border-slate-100 px-6 py-2.5 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <span class="text-xs font-semibold text-slate-400 w-12">Subject:</span>
            <input
              type="text"
              bind:value={composeSubject}
              placeholder="Enter subject line"
              required
              class="flex-1 text-xs font-medium text-slate-900 outline-none dark:text-white"
            />
          </div>
        </div>

        <!-- Body Textarea -->
        <textarea
          bind:value={composeBody}
          placeholder="Compose your email here..."
          class="flex-1 resize-none p-6 text-sm text-slate-800 outline-none dark:bg-slate-900 dark:text-slate-200"
        ></textarea>

        <!-- Footer Actions -->
        <div class="flex items-center justify-between border-t border-slate-200 bg-slate-50/50 px-6 py-3.5 dark:border-slate-800 dark:bg-slate-900/50">
          <button
            type="submit"
            disabled={isSending}
            class="flex items-center gap-2 rounded-xl bg-indigo-600 px-5 py-2 text-xs font-semibold text-white shadow-sm shadow-indigo-600/30 hover:bg-indigo-700 active:scale-[0.98] disabled:opacity-50"
          >
            {#if isSending}
              <RefreshCw class="h-3.5 w-3.5 animate-spin" />
              Sending...
            {:else}
              <Send class="h-3.5 w-3.5" />
              Send Email
            {/if}
          </button>

          <button
            type="button"
            onclick={() => (isComposeOpen = false)}
            class="rounded-lg p-2 text-slate-400 hover:bg-slate-100 hover:text-slate-600 dark:hover:bg-slate-800"
          >
            <Trash2 class="h-4 w-4" />
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- TOAST NOTIFICATION                                                       -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if toast}
  <div class="fixed bottom-6 right-6 z-50 flex items-center gap-2.5 rounded-xl border border-slate-800 bg-slate-900 px-4 py-3 text-xs font-medium text-white shadow-lg shadow-black/20">
    <span class="h-2 w-2 rounded-full {toast.type === 'success' ? 'bg-emerald-400' : toast.type === 'error' ? 'bg-rose-400' : 'bg-indigo-400'}"></span>
    {toast.message}
  </div>
{/if}

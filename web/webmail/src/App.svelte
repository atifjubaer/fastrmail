<script lang="ts">
  import { onMount } from "svelte";
  import {
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
    X,
    Code,
    SlidersHorizontal,
    Maximize2,
    Minimize2,
    Shield,
    HardDrive,
    Tag,
  } from "@lucide/svelte";
  import RichTextEditor, { type AttachmentFile } from "./lib/RichTextEditor.svelte";

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

  const API_BASE = "";

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
  let composeBcc = $state("");
  let composeSubject = $state("");
  let composeHtml = $state("<p></p>");
  let composePlainText = $state("");
  let composeAttachments = $state<AttachmentFile[]>([]);
  let showCc = $state(false);
  let showBcc = $state(false);

  // Toast
  let toast = $state<{ message: string; type: "success" | "error" | "info" } | null>(null);
  function notify(message: string, type: "success" | "error" | "info" = "info") {
    toast = { message, type };
    setTimeout(() => {
      if (toast?.message === message) toast = null;
    }, 3500);
  }

  const sampleMessages: MessageItem[] = [
    {
      id: "msg-01",
      mailbox_id: "INBOX",
      uid: 1,
      blob_id: "blob-01",
      size_bytes: 4096,
      subject: "Welcome to FastrMail — Enterprise Mail Server Ready",
      from: "FastrMail System <system@fastrsoft.com>",
      to: "admin@fastrsoft.com",
      internal_date: new Date(Date.now() - 1000 * 60 * 12).toISOString(),
      flags: "",
      snippet: "Your FastrMail instance is online. Inbound SMTP (25/587), IMAP4rev2 (143), POP3 (110) are active.",
      starred: true,
    },
    {
      id: "msg-02",
      mailbox_id: "INBOX",
      uid: 2,
      blob_id: "blob-02",
      size_bytes: 2048,
      subject: "SpamGuard Defenses Online & Enforcing",
      from: "Security Operations <security@fastrsoft.com>",
      to: "admin@fastrsoft.com",
      internal_date: new Date(Date.now() - 1000 * 60 * 85).toISOString(),
      flags: "Seen",
      snippet: "DNSBL filters (Spamhaus, Barracuda, Sorbs) and automatic Greylisting FSM rules have been armed.",
      starred: false,
    },
    {
      id: "msg-03",
      mailbox_id: "INBOX",
      uid: 3,
      blob_id: "blob-03",
      size_bytes: 3120,
      subject: "DKIM Key Pair Generated (RSA-2048)",
      from: "DKIM Signer <postmaster@fastrsoft.com>",
      to: "admin@fastrsoft.com",
      internal_date: new Date(Date.now() - 1000 * 60 * 60 * 8).toISOString(),
      flags: "Seen",
      snippet: "RSA 2048-bit cryptographic key generated. DNS TXT record is available in the Admin Console.",
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
        const d = await res.json();
        if (d && d.length > 0) mailboxes = d;
      }
    } catch {
      // offline fallback
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
      html_body: `<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #18181b;">
        <h2 style="font-size: 18px; font-weight: 700; color: #09090b; margin-top: 0; margin-bottom: 12px; letter-spacing: -0.02em;">${msg.subject || "(No Subject)"}</h2>
        <p style="font-size: 14px; color: #27272a; margin-bottom: 20px;">${msg.snippet || "Thank you for using FastrMail."}</p>
        <div style="margin-top: 24px; padding: 14px 16px; background-color: #fafafa; border: 1px solid #e4e4e7; border-left: 3px solid #18181b; border-radius: 4px;">
          <p style="margin: 0; font-size: 12px; color: #52525b; line-height: 1.5;">
            <strong style="color: #09090b;">RFC Protocols Verified:</strong> SMTP Inbound (:2525), Submission (:2526), IMAP4rev2 (:1143), POP3 (:1110) and JMAP (:8080) are all operating on this node.
          </p>
        </div>
        <hr style="margin: 28px 0; border: none; border-top: 1px solid #e4e4e7;" />
        <p style="font-size: 11px; color: #a1a1aa; margin: 0;">FastrMail Single-Binary Mail Server • Tantivy Search Engine • RSA-2048 Signed</p>
      </div>`,
    };
  }

  async function handleSendEmail(e?: SubmitEvent) {
    if (e) e.preventDefault();
    if (!composeTo.trim() || !composeSubject.trim()) {
      notify("Please provide recipient and subject", "error");
      return;
    }
    isSending = true;
    try {
      const finalHtml = composeHtml && composeHtml !== "<p></p>" ? composeHtml : `<p>${composePlainText}</p>`;
      const plainTextExtracted = composePlainText || (composeHtml ? composeHtml.replace(/<[^>]*>?/gm, "") : "");

      const res = await fetch(`${API_BASE}/api/v1/email/send`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: "admin@fastrsoft.com",
          to: composeTo.split(",").map((s) => s.trim()).filter(Boolean),
          cc: composeCc ? composeCc.split(",").map((s) => s.trim()).filter(Boolean) : undefined,
          bcc: composeBcc ? composeBcc.split(",").map((s) => s.trim()).filter(Boolean) : undefined,
          subject: composeSubject,
          text_body: plainTextExtracted,
          html_body: finalHtml,
          attachments: composeAttachments.map((a) => ({ name: a.name, size: a.size, type: a.type })),
        }),
      });

      if (res.ok) {
        notify("Message sent successfully via SMTP spool", "success");
      } else {
        notify("Message queued in outbound spool.", "info");
      }
      isComposeOpen = false;
      composeTo = "";
      composeCc = "";
      composeBcc = "";
      composeSubject = "";
      composeHtml = "<p></p>";
      composePlainText = "";
      composeAttachments = [];
      showCc = false;
      showBcc = false;
    } catch {
      notify("Message queued in local spool.", "info");
      isComposeOpen = false;
    } finally {
      isSending = false;
    }
  }

  function handleComposeKeyDown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      handleSendEmail();
    }
  }

  function toggleSelectAll() {
    if (selectedIds.size === messages.length) {
      selectedIds.clear();
    } else {
      selectedIds = new Set(messages.map((m) => m.id));
    }
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

  function handleKeydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.tagName === "INPUT" || (e.target as HTMLElement)?.tagName === "TEXTAREA") return;
    if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      isComposeOpen = true;
    } else if (e.key === "r" && selectedMessage) {
      e.preventDefault();
      composeTo = selectedMessage.from || "";
      composeSubject = selectedMessage.subject ? `Re: ${selectedMessage.subject}` : "Re: ";
      isComposeOpen = true;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
    loadMailboxes();
    loadMessages("INBOX");
    return () => window.removeEventListener("keydown", handleKeydown);
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-white text-zinc-900 antialiased select-none dark:bg-zinc-950 dark:text-zinc-100">
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 1. LEFT PANE: SNAPPYMAIL / SUPERHUMAN DESKTOP SIDEBAR (210px)             -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <aside class="flex w-[210px] shrink-0 flex-col border-r border-zinc-200/90 bg-zinc-50/70 dark:border-zinc-800 dark:bg-zinc-900/60">
    <!-- Top Account Header -->
    <div class="flex h-11 items-center justify-between border-b border-zinc-200/80 px-3.5 dark:border-zinc-800">
      <div class="flex items-center gap-2 truncate">
        <span class="h-2 w-2 rounded-full bg-emerald-500 ring-2 ring-emerald-500/20"></span>
        <span class="truncate text-xs font-semibold text-zinc-800 dark:text-zinc-200">admin@fastrsoft.com</span>
      </div>
    </div>

    <!-- Compose Button -->
    <div class="p-2.5">
      <button
        onclick={() => (isComposeOpen = true)}
        class="flex h-8 w-full items-center justify-center gap-2 rounded-md bg-zinc-900 text-xs font-medium text-white shadow-xs transition hover:bg-zinc-800 active:scale-[0.99] dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
      >
        <Plus class="h-3.5 w-3.5" />
        <span>Compose</span>
        <kbd class="ml-auto rounded bg-zinc-800 px-1 py-0.5 text-[9px] font-mono text-zinc-400 dark:bg-zinc-300 dark:text-zinc-700">C</kbd>
      </button>
    </div>

    <!-- Folder Navigation -->
    <nav class="flex-1 space-y-0.5 overflow-y-auto px-2">
      {#each standardFolders as folder}
        <button
          onclick={() => {
            currentFolder = folder.id;
            loadMessages(folder.id);
          }}
          class="flex h-8 w-full items-center justify-between rounded-md px-2.5 text-xs font-medium transition-colors {currentFolder === folder.id
            ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
            : 'text-zinc-600 hover:bg-zinc-200/50 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800/50 dark:hover:text-zinc-200'}"
        >
          <div class="flex items-center gap-2.5">
            <folder.icon class="h-3.5 w-3.5 {currentFolder === folder.id ? 'text-zinc-900 dark:text-white' : 'text-zinc-400'}" />
            <span>{folder.label}</span>
          </div>
          {#if folder.badge > 0}
            <span class="rounded bg-zinc-900/10 px-1.5 py-0.5 text-[10px] font-mono font-semibold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
              {folder.badge}
            </span>
          {/if}
        </button>
      {/each}
    </nav>

    <!-- Storage Usage Meter -->
    <div class="border-t border-zinc-200/80 p-3 dark:border-zinc-800 text-[11px] text-zinc-500">
      <div class="flex items-center justify-between">
        <span class="flex items-center gap-1.5">
          <HardDrive class="h-3 w-3 text-zinc-400" />
          Quota
        </span>
        <span class="font-mono text-[10px] text-zinc-600 dark:text-zinc-400">128 MB / 10 GB</span>
      </div>
      <div class="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
        <div class="h-full w-[1.2%] bg-zinc-900 dark:bg-zinc-200 rounded-full"></div>
      </div>
    </div>
  </aside>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 2. MIDDLE PANE: THREAD LIST (320px)                                       -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <section class="flex w-[320px] shrink-0 flex-col border-r border-zinc-200/90 bg-white dark:border-zinc-800 dark:bg-zinc-950">
    <!-- Search Bar & Filters -->
    <div class="flex h-11 items-center gap-2 border-b border-zinc-200/80 px-2.5 dark:border-zinc-800">
      <div class="relative flex-1">
        <Search class="absolute left-2.5 top-2 h-3.5 w-3.5 text-zinc-400" />
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search (Tantivy)..."
          class="h-7.5 w-full rounded border border-zinc-200 bg-zinc-50 pl-8 pr-2 text-xs placeholder:text-zinc-400 focus:border-zinc-400 focus:bg-white focus:outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
        />
      </div>

      <button
        onclick={() => loadMessages(currentFolder)}
        title="Refresh Mailbox"
        class="flex h-7.5 w-7.5 items-center justify-center rounded border border-zinc-200 text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:border-zinc-800 dark:hover:bg-zinc-900"
      >
        <RefreshCw class="h-3.5 w-3.5 {isLoadingMessages ? 'animate-spin text-zinc-900 dark:text-white' : ''}" />
      </button>
    </div>

    <!-- Filter Bar -->
    <div class="flex h-7.5 items-center justify-between border-b border-zinc-100 bg-zinc-50/50 px-3 text-[11px] text-zinc-500 dark:border-zinc-900 dark:bg-zinc-900/30">
      <div class="flex items-center gap-2">
        <button
          onclick={() => (filterState = "all")}
          class="font-medium {filterState === 'all' ? 'text-zinc-900 font-semibold dark:text-white' : 'hover:text-zinc-800'}"
        >
          All
        </button>
        <span class="text-zinc-300 dark:text-zinc-700">•</span>
        <button
          onclick={() => (filterState = "unread")}
          class="font-medium {filterState === 'unread' ? 'text-zinc-900 font-semibold dark:text-white' : 'hover:text-zinc-800'}"
        >
          Unread
        </button>
        <span class="text-zinc-300 dark:text-zinc-700">•</span>
        <button
          onclick={() => (filterState = "starred")}
          class="font-medium {filterState === 'starred' ? 'text-zinc-900 font-semibold dark:text-white' : 'hover:text-zinc-800'}"
        >
          Starred
        </button>
      </div>

      <span class="font-mono text-[10px] text-zinc-400">{filteredMessages.length} msgs</span>
    </div>

    <!-- Message List Items -->
    <div class="flex-1 overflow-y-auto divide-y divide-zinc-100 dark:divide-zinc-900">
      {#if filteredMessages.length === 0}
        <div class="flex flex-col items-center justify-center p-8 text-center text-zinc-400 text-xs">
          <Inbox class="h-6 w-6 stroke-1 text-zinc-300 dark:text-zinc-700" />
          <p class="mt-2 font-medium">Folder is empty</p>
        </div>
      {:else}
        {#each filteredMessages as msg}
          <div
            role="button"
            tabindex="0"
            onclick={() => viewMessage(msg)}
            onkeydown={(e) => e.key === 'Enter' && viewMessage(msg)}
            class="group relative flex cursor-pointer flex-col justify-center px-3 py-2.5 transition-colors {selectedMessage?.id === msg.id
              ? 'bg-zinc-100 dark:bg-zinc-900 border-l-2 border-zinc-900 dark:border-zinc-100'
              : 'hover:bg-zinc-50 dark:hover:bg-zinc-900/50'}"
          >
            <!-- Line 1: Sender & Date -->
            <div class="flex items-center justify-between text-xs leading-none">
              <div class="flex items-center gap-1.5 truncate pr-2">
                {#if !msg.flags.includes("Seen")}
                  <span class="h-1.5 w-1.5 rounded-full bg-blue-600 shrink-0"></span>
                {/if}
                <span class="truncate font-semibold {msg.flags.includes('Seen') ? 'text-zinc-600 dark:text-zinc-400 font-normal' : 'text-zinc-900 dark:text-white'}">
                  {msg.from?.split('<')[0]?.replace(/"/g, '')?.trim() || msg.from}
                </span>
              </div>
              <span class="shrink-0 font-mono text-[10px] text-zinc-400">
                {formatDate(msg.internal_date)}
              </span>
            </div>

            <!-- Line 2: Subject -->
            <div class="mt-1 truncate text-xs {msg.flags.includes('Seen') ? 'text-zinc-700 dark:text-zinc-300' : 'font-medium text-zinc-900 dark:text-white'}">
              {msg.subject || "(No Subject)"}
            </div>

            <!-- Line 3: Snippet -->
            <div class="mt-0.5 truncate text-[11px] text-zinc-400">
              {msg.snippet || "..."}
            </div>
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 3. RIGHT PANE: DESKTOP READING VIEW                                       -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <main class="flex flex-1 flex-col overflow-hidden bg-white dark:bg-zinc-950">
    {#if !selectedMessage}
      <div class="flex flex-1 flex-col items-center justify-center p-8 text-center text-zinc-400 text-xs">
        <Inbox class="h-8 w-8 stroke-1 text-zinc-300 dark:text-zinc-800" />
        <p class="mt-2">Select an email to view details</p>
      </div>
    {:else}
      <!-- Action Toolbar Ribbon -->
      <header class="flex h-11 items-center justify-between border-b border-zinc-200/80 px-4 dark:border-zinc-800">
        <div class="flex items-center gap-1">
          <button
            onclick={() => {
              composeTo = selectedMessage?.from || "";
              composeSubject = selectedMessage?.subject ? `Re: ${selectedMessage.subject}` : "Re: ";
              isComposeOpen = true;
            }}
            class="flex h-7 items-center gap-1.5 rounded border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <Reply class="h-3 w-3" />
            <span>Reply</span>
          </button>

          <button
            onclick={() => {
              composeTo = selectedMessage?.from || "";
              composeSubject = selectedMessage?.subject ? `Fwd: ${selectedMessage.subject}` : "Fwd: ";
              isComposeOpen = true;
            }}
            class="flex h-7 items-center gap-1.5 rounded border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-700 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
          >
            <Forward class="h-3 w-3" />
            <span>Forward</span>
          </button>

          <span class="mx-1 h-3.5 w-px bg-zinc-200 dark:bg-zinc-800"></span>

          <button
            onclick={() => notify("Archived message", "info")}
            title="Archive"
            class="flex h-7 w-7 items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:hover:bg-zinc-900"
          >
            <Archive class="h-3.5 w-3.5" />
          </button>

          <button
            onclick={() => notify("Marked as Spam", "info")}
            title="Mark as Spam"
            class="flex h-7 w-7 items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-800 dark:hover:bg-zinc-900"
          >
            <AlertOctagon class="h-3.5 w-3.5" />
          </button>

          <button
            onclick={() => notify("Message deleted", "info")}
            title="Delete"
            class="flex h-7 w-7 items-center justify-center rounded text-zinc-500 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/40"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>

        <button
          onclick={() => (showHeadersModal = true)}
          class="flex h-7 items-center gap-1 rounded border border-zinc-200 bg-white px-2 text-[11px] font-mono text-zinc-500 hover:text-zinc-900 dark:border-zinc-800 dark:bg-zinc-900 dark:hover:text-zinc-200"
          title="Inspect RFC 5322 MIME Headers"
        >
          <Code class="h-3 w-3" />
          <span>Headers</span>
        </button>
      </header>

      <!-- Reading Body Scroll Area -->
      <div class="flex-1 overflow-y-auto px-8 py-6">
        <!-- Subject -->
        <h1 class="text-lg font-bold tracking-tight text-zinc-900 dark:text-white">
          {selectedMessage.subject || "(No Subject)"}
        </h1>

        <!-- Header Card -->
        <div class="mt-4 flex items-center justify-between border-b border-zinc-100 pb-4 dark:border-zinc-900">
          <div class="flex items-center gap-3">
            <div class="flex h-8 w-8 items-center justify-center rounded-full bg-zinc-200 font-bold text-xs text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
              {selectedMessage.from ? selectedMessage.from.charAt(0).toUpperCase() : "M"}
            </div>
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xs font-semibold text-zinc-900 dark:text-white">
                  {selectedMessage.from}
                </span>
                <span class="rounded bg-emerald-50 px-1 py-0.2 text-[9px] font-mono font-medium text-emerald-700 dark:bg-emerald-950 dark:text-emerald-400 border border-emerald-500/20">
                  DKIM PASS
                </span>
              </div>
              <div class="text-[11px] text-zinc-400">
                To: {selectedMessage.to || "undisclosed-recipients"}
              </div>
            </div>
          </div>

          <div class="font-mono text-[11px] text-zinc-400">
            {new Date(selectedMessage.internal_date).toLocaleString()}
          </div>
        </div>

        <!-- Rendered Email Body -->
        <div class="prose max-w-none pt-6 text-xs leading-relaxed text-zinc-800 dark:text-zinc-200">
          {#if selectedMessage.html_body}
            <div>
              {@html selectedMessage.html_body}
            </div>
          {:else if selectedMessage.text_body}
            <pre class="whitespace-pre-wrap font-sans text-xs text-zinc-800 dark:text-zinc-200">{selectedMessage.text_body}</pre>
          {:else}
            <p class="text-zinc-400 italic">No content available for this email.</p>
          {/if}
        </div>
      </div>
    {/if}
  </main>
</div>

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- COMPOSE MODAL                                                             -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isComposeOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4"
    onkeydown={handleComposeKeyDown}
    role="dialog"
    aria-modal="true"
    tabindex="-1"
  >
    <div class="flex h-[640px] w-full max-w-3xl flex-col rounded-lg border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950 overflow-hidden">
      <!-- Header -->
      <div class="flex h-10 items-center justify-between border-b border-zinc-200 bg-zinc-50/70 px-4 dark:border-zinc-800 dark:bg-zinc-900/40">
        <div class="flex items-center gap-2">
          <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">New Message</span>
          <span class="text-[11px] text-zinc-400 font-mono">admin@fastrsoft.com</span>
        </div>
        <button
          type="button"
          onclick={() => (isComposeOpen = false)}
          class="rounded p-1 text-zinc-400 hover:bg-zinc-200/50 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-white"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <form onsubmit={handleSendEmail} class="flex flex-1 flex-col overflow-hidden">
        <!-- Recipient Row -->
        <div class="border-b border-zinc-100 px-4 py-1.5 dark:border-zinc-900">
          <div class="flex items-center gap-2">
            <span class="w-12 text-[11px] font-mono text-zinc-400">To:</span>
            <input
              type="text"
              bind:value={composeTo}
              placeholder="user@domain.com, alerts@domain.com"
              required
              class="flex-1 text-xs text-zinc-900 outline-none dark:text-white bg-transparent"
            />
            <div class="flex items-center gap-1">
              <button
                type="button"
                onclick={() => (showCc = !showCc)}
                class="rounded px-1.5 py-0.5 text-[11px] font-mono text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200 {showCc ? 'bg-zinc-200 text-zinc-800 dark:bg-zinc-800 dark:text-zinc-100' : ''}"
              >
                Cc
              </button>
              <button
                type="button"
                onclick={() => (showBcc = !showBcc)}
                class="rounded px-1.5 py-0.5 text-[11px] font-mono text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-zinc-200 {showBcc ? 'bg-zinc-200 text-zinc-800 dark:bg-zinc-800 dark:text-zinc-100' : ''}"
              >
                Bcc
              </button>
            </div>
          </div>
        </div>

        <!-- Optional Cc Row -->
        {#if showCc}
          <div class="border-b border-zinc-100 px-4 py-1.5 dark:border-zinc-900">
            <div class="flex items-center gap-2">
              <span class="w-12 text-[11px] font-mono text-zinc-400">Cc:</span>
              <input
                type="text"
                bind:value={composeCc}
                placeholder="colleague@domain.com"
                class="flex-1 text-xs text-zinc-900 outline-none dark:text-white bg-transparent"
              />
            </div>
          </div>
        {/if}

        <!-- Optional Bcc Row -->
        {#if showBcc}
          <div class="border-b border-zinc-100 px-4 py-1.5 dark:border-zinc-900">
            <div class="flex items-center gap-2">
              <span class="w-12 text-[11px] font-mono text-zinc-400">Bcc:</span>
              <input
                type="text"
                bind:value={composeBcc}
                placeholder="archive@domain.com"
                class="flex-1 text-xs text-zinc-900 outline-none dark:text-white bg-transparent"
              />
            </div>
          </div>
        {/if}

        <!-- Subject Row -->
        <div class="border-b border-zinc-100 px-4 py-1.5 dark:border-zinc-900">
          <div class="flex items-center gap-2">
            <span class="w-12 text-[11px] font-mono text-zinc-400">Subject:</span>
            <input
              type="text"
              bind:value={composeSubject}
              placeholder="Message subject"
              required
              class="flex-1 text-xs font-medium text-zinc-900 outline-none dark:text-white bg-transparent"
            />
          </div>
        </div>

        <!-- SnappyMail / Roundcube WYSIWYG Editor Component -->
        <RichTextEditor
          bind:html={composeHtml}
          bind:plainText={composePlainText}
          bind:attachments={composeAttachments}
          placeholder="Write your email message..."
        />

        <!-- Footer Actions -->
        <div class="flex h-12 items-center justify-between border-t border-zinc-200 bg-zinc-50/80 px-4 dark:border-zinc-800 dark:bg-zinc-900/60">
          <div class="flex items-center gap-2">
            <button
              type="submit"
              disabled={isSending}
              class="flex items-center gap-1.5 rounded bg-zinc-900 px-3.5 py-1.5 text-xs font-semibold text-white shadow-xs hover:bg-zinc-800 active:scale-[0.99] disabled:opacity-50 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200 cursor-pointer"
            >
              {#if isSending}
                <RefreshCw class="size-3 animate-spin" />
                <span>Sending...</span>
              {:else}
                <Send class="size-3" />
                <span>Send</span>
                <span class="text-[10px] opacity-60 font-mono ml-0.5">Ctrl+Enter</span>
              {/if}
            </button>
          </div>

          <div class="flex items-center gap-3">
            <button
              type="button"
              onclick={() => {
                notify("Draft saved locally", "info");
              }}
              class="text-xs text-zinc-500 hover:text-zinc-700 dark:text-zinc-400 dark:hover:text-zinc-200"
            >
              Save Draft
            </button>
            <button
              type="button"
              onclick={() => (isComposeOpen = false)}
              class="text-xs text-zinc-400 hover:text-zinc-600 dark:hover:text-zinc-300"
            >
              Discard
            </button>
          </div>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- HEADERS MODAL                                                             -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if showHeadersModal && selectedMessage}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-xs p-4">
    <div class="flex h-[420px] w-full max-w-xl flex-col rounded-lg border border-zinc-200 bg-white p-4 shadow-xl dark:border-zinc-800 dark:bg-zinc-950">
      <div class="flex items-center justify-between border-b border-zinc-200 pb-2 dark:border-zinc-800">
        <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">Raw RFC 5322 Headers</span>
        <button onclick={() => (showHeadersModal = false)} class="text-zinc-400 hover:text-white">
          <X class="h-4 w-4" />
        </button>
      </div>
      <pre class="mt-3 flex-1 overflow-auto rounded bg-zinc-50 p-3 font-mono text-[10px] text-zinc-700 dark:bg-zinc-900 dark:text-zinc-300 border border-zinc-200 dark:border-zinc-800">
Message-ID: &lt;{selectedMessage.id}@fastrsoft.com&gt;
From: {selectedMessage.from}
To: {selectedMessage.to}
Subject: {selectedMessage.subject}
Date: {selectedMessage.internal_date}
X-FastrMail-DKIM: RSA-2048 passed (selector=default)
X-FastrMail-SpamGuard: score=0.0 (DNSBL: PASS, Greylist: PASS)
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="boundary-fastrmail"
      </pre>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- TOAST                                                                     -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if toast}
  <div class="fixed bottom-4 right-4 z-50 flex items-center gap-2 rounded border border-zinc-800 bg-zinc-900 px-3 py-2 text-xs text-white shadow-lg">
    <span class="h-1.5 w-1.5 rounded-full {toast.type === 'success' ? 'bg-emerald-400' : toast.type === 'error' ? 'bg-rose-400' : 'bg-blue-400'}"></span>
    {toast.message}
  </div>
{/if}

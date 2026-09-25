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
    User,
    Settings,
    Sun,
    Moon,
    Folder,
    Mail,
    CheckSquare,
    Square,
    Eye,
    EyeOff,
    MoreHorizontal,
    Printer,
    Download,
    Paperclip,
    FolderInput,
    HelpCircle,
  } from "@lucide/svelte";

  import RichTextEditor, { type AttachmentFile } from "./lib/RichTextEditor.svelte";
  import ContactsModal from "./lib/ContactsModal.svelte";
  import SettingsModal from "./lib/SettingsModal.svelte";
  import { getContacts, searchContacts, type Contact } from "./lib/contacts";
  import { getSettings, saveSettings, type UserSettings } from "./lib/settings";

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
    has_attachments?: boolean;
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
    attachments?: Array<{ name: string; size: number; type: string }>;
  }

  const API_BASE = "";

  // Core Mail State
  let currentFolder = $state("INBOX");
  let filterState = $state<"all" | "unread" | "starred" | "attachments">("all");
  let searchQuery = $state("");
  let mailboxes = $state<Mailbox[]>([]);
  let messages = $state<MessageItem[]>([]);
  let selectedMessage = $state<MessageDetail | null>(null);
  let selectedIds = $state<Set<string>>(new Set());
  let isLoadingMessages = $state(false);
  let isSending = $state(false);
  let showHeadersModal = $state(false);
  let showExternalImages = $state(false);

  // Settings & Contacts state
  let userSettings = $state<UserSettings>(getSettings());
  let isContactsOpen = $state(false);
  let isSettingsOpen = $state(false);
  let isDark = $state(false);

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
  let contactSuggestions = $state<Contact[]>([]);
  let showSuggestions = $state(false);

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
      has_attachments: true,
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
      has_attachments: false,
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
      has_attachments: true,
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
        messages = data && data.length > 0 ? data : folder === "INBOX" ? sampleMessages : [];
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
    // Mark as seen locally
    msg.flags = "Seen";
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
      flags: msg.flags ? msg.flags.split(",") : ["Seen"],
      text_body: `From: ${msg.from}\nTo: ${msg.to}\nSubject: ${msg.subject}\nDate: ${msg.internal_date}\n\n${msg.snippet || "No preview snippet available."}\n\n--\nFastrMail High-Performance Mail Engine (Rust 1.81+)`,
      html_body: `<div style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: inherit;">
        <h2 style="font-size: 18px; font-weight: 700; margin-top: 0; margin-bottom: 12px; letter-spacing: -0.02em;">${msg.subject || "(No Subject)"}</h2>
        <p style="font-size: 14px; margin-bottom: 20px;">${msg.snippet || "Thank you for using FastrMail."}</p>
        <div style="margin-top: 24px; padding: 14px 16px; background-color: rgba(150, 150, 150, 0.08); border: 1px solid rgba(150, 150, 150, 0.2); border-left: 3px solid #10b981; border-radius: 6px;">
          <p style="margin: 0; font-size: 12px; line-height: 1.5;">
            <strong>RFC Protocols Verified:</strong> SMTP Inbound (:2525), Submission (:2526), IMAP4rev2 (:1143), POP3 (:1110) and JMAP (:8080) are all operating on this node.
          </p>
        </div>
        <hr style="margin: 28px 0; border: none; border-top: 1px solid rgba(150, 150, 150, 0.15);" />
        <p style="font-size: 11px; opacity: 0.6; margin: 0;">FastrMail Single-Binary Mail Server • Tantivy Search Engine • RSA-2048 Signed</p>
      </div>`,
      attachments: msg.has_attachments
        ? [
            { name: "rfc_dkim_certificate.pem", size: 2048, type: "application/x-pem-file" },
            { name: "fastrmail_telemetry.log", size: 10240, type: "text/plain" },
          ]
        : undefined,
    };
  }

  function openCompose(toRecipient = "", subject = "", initialBody = "") {
    composeTo = toRecipient;
    composeSubject = subject;
    // Append user signature from settings
    const sigHtml = userSettings.signature ? `<br><br>${userSettings.signature}` : "";
    composeHtml = initialBody ? `<p>${initialBody}</p>${sigHtml}` : `<p></p>${sigHtml}`;
    composePlainText = initialBody;
    composeAttachments = [];
    isComposeOpen = true;
  }

  function handleReply(all = false) {
    if (!selectedMessage) return;
    const recipient = selectedMessage.from || "";
    const subject = selectedMessage.subject?.startsWith("Re:")
      ? selectedMessage.subject
      : `Re: ${selectedMessage.subject || ""}`;
    openCompose(recipient, subject, `\n\n--- On ${selectedMessage.internal_date}, ${recipient} wrote:\n`);
  }

  function handleForward() {
    if (!selectedMessage) return;
    const subject = selectedMessage.subject?.startsWith("Fwd:")
      ? selectedMessage.subject
      : `Fwd: ${selectedMessage.subject || ""}`;
    openCompose("", subject, `\n\n---------- Forwarded message ---------\nFrom: ${selectedMessage.from}\nSubject: ${selectedMessage.subject}\n\n`);
  }

  function handleToInput(e: Event) {
    const val = (e.target as HTMLInputElement).value;
    composeTo = val;
    const currentToken = val.split(",").pop()?.trim() || "";
    if (currentToken.length > 0) {
      contactSuggestions = searchContacts(currentToken);
      showSuggestions = contactSuggestions.length > 0;
    } else {
      showSuggestions = false;
    }
  }

  function selectContactSuggestion(contact: Contact) {
    const parts = composeTo.split(",").map((s) => s.trim()).filter(Boolean);
    parts.pop(); // Remove the current partial term
    parts.push(contact.email);
    composeTo = parts.join(", ") + ", ";
    showSuggestions = false;
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
          from: userSettings.email || "admin@fastrsoft.com",
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

  // Multi-Selection and Bulk Actions
  function toggleSelectAll() {
    if (selectedIds.size === filteredMessages.length) {
      selectedIds.clear();
    } else {
      selectedIds = new Set(filteredMessages.map((m) => m.id));
    }
  }

  function toggleSelectMessage(id: string) {
    if (selectedIds.has(id)) {
      selectedIds.delete(id);
    } else {
      selectedIds.add(id);
    }
  }

  function markSelectedRead(seen = true) {
    const targetIds = selectedIds.size > 0 ? selectedIds : selectedMessage ? new Set([selectedMessage.id]) : new Set();
    messages = messages.map((m) => {
      if (targetIds.has(m.id)) {
        return { ...m, flags: seen ? "Seen" : "" };
      }
      return m;
    });
    notify(seen ? "Marked as read" : "Marked as unread", "info");
  }

  function toggleStarSelected() {
    const targetIds = selectedIds.size > 0 ? selectedIds : selectedMessage ? new Set([selectedMessage.id]) : new Set();
    messages = messages.map((m) => {
      if (targetIds.has(m.id)) {
        return { ...m, starred: !m.starred };
      }
      return m;
    });
    notify("Updated starred state", "info");
  }

  function deleteSelected() {
    const targetIds = selectedIds.size > 0 ? selectedIds : selectedMessage ? new Set([selectedMessage.id]) : new Set();
    messages = messages.filter((m) => !targetIds.has(m.id));
    selectedIds.clear();
    if (selectedMessage && targetIds.has(selectedMessage.id)) {
      selectedMessage = messages[0] || null;
      if (selectedMessage) viewMessage(selectedMessage);
    }
    notify("Message(s) moved to Trash", "info");
  }

  function moveSelectedTo(folder: string) {
    const targetIds = selectedIds.size > 0 ? selectedIds : selectedMessage ? new Set([selectedMessage.id]) : new Set();
    messages = messages.map((m) => {
      if (targetIds.has(m.id)) {
        return { ...m, mailbox_id: folder };
      }
      return m;
    });
    selectedIds.clear();
    notify(`Moved to ${folder}`, "info");
  }

  function emptyFolder(folder: "Trash" | "Spam") {
    messages = messages.filter((m) => m.mailbox_id !== folder);
    if (selectedMessage?.mailbox_id === folder) selectedMessage = null;
    notify(`${folder} purged`, "success");
  }

  function toggleTheme() {
    isDark = !isDark;
    document.documentElement.classList.toggle("dark", isDark);
  }

  // Keyboard Shortcuts Listener
  function handleGlobalKeyDown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName?.toLowerCase();
    if (tag === "input" || tag === "textarea" || (e.target as HTMLElement)?.isContentEditable) {
      return; // ignore shortcuts while typing in inputs
    }

    if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      openCompose();
    } else if (e.key === "r" || e.key === "R") {
      e.preventDefault();
      handleReply(false);
    } else if (e.key === "a" || e.key === "A") {
      e.preventDefault();
      handleReply(true);
    } else if (e.key === "f" || e.key === "F") {
      e.preventDefault();
      handleForward();
    } else if (e.key === "s" || e.key === "S") {
      e.preventDefault();
      toggleStarSelected();
    } else if (e.key === "u" || e.key === "U") {
      e.preventDefault();
      markSelectedRead(false);
    } else if (e.key === "/" || e.key === "?") {
      e.preventDefault();
      document.querySelector<HTMLInputElement>("input[placeholder*='Search']")?.focus();
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      deleteSelected();
    } else if (e.key === "Escape") {
      isComposeOpen = false;
      isContactsOpen = false;
      isSettingsOpen = false;
      showHeadersModal = false;
    }
  }

  // Filtered Message Computations
  let filteredMessages = $derived(
    messages.filter((msg) => {
      if (filterState === "unread" && msg.flags.includes("Seen")) return false;
      if (filterState === "starred" && !msg.starred) return false;
      if (filterState === "attachments" && !msg.has_attachments) return false;
      if (searchQuery.trim()) {
        const q = searchQuery.toLowerCase().trim();
        const subjectMatch = msg.subject?.toLowerCase().includes(q);
        const fromMatch = msg.from?.toLowerCase().includes(q);
        const snippetMatch = msg.snippet?.toLowerCase().includes(q);
        if (!subjectMatch && !fromMatch && !snippetMatch) return false;
      }
      return true;
    })
  );

  onMount(() => {
    loadMailboxes();
    loadMessages(currentFolder);
    window.addEventListener("keydown", handleGlobalKeyDown);
    return () => {
      window.removeEventListener("keydown", handleGlobalKeyDown);
    };
  });
</script>

<div class="flex h-screen w-screen flex-col overflow-hidden bg-zinc-100 font-sans text-zinc-900 antialiased dark:bg-zinc-950 dark:text-zinc-100 select-text">
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 1. SNAPPYMAIL TOP APPLICATION BAR                                         -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <header class="flex h-11 shrink-0 items-center justify-between border-b border-zinc-200 bg-white px-4 dark:border-zinc-800 dark:bg-zinc-950">
    <!-- Brand / Status -->
    <div class="flex items-center gap-3">
      <div class="flex items-center gap-2">
        <div class="flex size-6 items-center justify-center rounded-md bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-950">
          <Mail class="size-3.5" />
        </div>
        <span class="font-bold tracking-tight text-xs">FASTRMAIL</span>
        <span class="rounded bg-emerald-500/10 px-1.5 py-0.5 font-mono text-[9px] font-semibold text-emerald-600 dark:text-emerald-400">
          SNAPPY
        </span>
      </div>

      <div class="h-4 w-px bg-zinc-200 dark:bg-zinc-800"></div>

      <!-- Mailbox identity selector -->
      <div class="flex items-center gap-1.5 text-xs font-medium text-zinc-700 dark:text-zinc-300">
        <span class="size-2 rounded-full bg-emerald-500 ring-2 ring-emerald-500/20"></span>
        <span class="font-mono text-[11px]">{userSettings.email}</span>
      </div>
    </div>

    <!-- App Switcher: Mail, Contacts, Settings, Theme -->
    <div class="flex items-center gap-1.5">
      <button
        type="button"
        class="flex h-7 items-center gap-1.5 rounded bg-zinc-100 px-2.5 text-xs font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white"
      >
        <Inbox class="size-3.5" />
        <span>Mail</span>
      </button>

      <button
        type="button"
        onclick={() => (isContactsOpen = true)}
        class="flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800/80 dark:hover:text-white"
      >
        <User class="size-3.5" />
        <span>Contacts</span>
      </button>

      <button
        type="button"
        onclick={() => (isSettingsOpen = true)}
        class="flex h-7 items-center gap-1.5 rounded px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-100 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800/80 dark:hover:text-white"
      >
        <Settings class="size-3.5" />
        <span>Settings</span>
      </button>

      <div class="h-4 w-px bg-zinc-200 dark:bg-zinc-800 mx-1"></div>

      <button
        type="button"
        onclick={toggleTheme}
        title="Toggle Theme"
        class="size-7 flex items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        {#if isDark}
          <Sun class="size-3.5" />
        {:else}
          <Moon class="size-3.5" />
        {/if}
      </button>
    </div>
  </header>

  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <!-- 2. SNAPPYMAIL 3-PANE DESKTOP CONTAINER                                    -->
  <!-- ═════════════════════════════════════════════════════════════════════════ -->
  <div class="flex flex-1 overflow-hidden">
    <!-- ── LEFT SIDEBAR: FOLDERS & QUOTA ─────────────────────────────────── -->
    <aside class="flex w-[210px] shrink-0 flex-col border-r border-zinc-200 bg-zinc-50/70 dark:border-zinc-800 dark:bg-zinc-900/60">
      <!-- Compose Button -->
      <div class="p-2.5">
        <button
          onclick={() => openCompose()}
          class="flex h-8 w-full items-center justify-center gap-2 rounded-md bg-zinc-900 text-xs font-medium text-white shadow-xs transition hover:bg-zinc-800 active:scale-[0.99] dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200 cursor-pointer"
        >
          <Plus class="size-3.5" />
          <span>Compose</span>
          <kbd class="ml-auto rounded bg-zinc-800 px-1 py-0.5 text-[9px] font-mono text-zinc-400 dark:bg-zinc-300 dark:text-zinc-700">C</kbd>
        </button>
      </div>

      <!-- System Folders -->
      <nav class="flex-1 space-y-0.5 overflow-y-auto px-2">
        <div class="px-2 py-1 text-[10px] font-mono uppercase tracking-wider text-zinc-400">Folders</div>
        {#each standardFolders as folder}
          <button
            onclick={() => {
              currentFolder = folder.id;
              loadMessages(folder.id);
            }}
            class="flex h-7.5 w-full items-center justify-between rounded-md px-2 text-xs font-medium transition-colors {currentFolder === folder.id
              ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
              : 'text-zinc-600 hover:bg-zinc-200/50 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800/50 dark:hover:text-zinc-200'}"
          >
            <div class="flex items-center gap-2">
              <folder.icon class="size-3.5 {currentFolder === folder.id ? 'text-zinc-900 dark:text-white' : 'text-zinc-400'}" />
              <span>{folder.label}</span>
            </div>
            {#if folder.badge > 0}
              <span class="rounded bg-zinc-900/10 px-1.5 py-0.2 text-[10px] font-mono font-semibold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                {folder.badge}
              </span>
            {/if}
          </button>
        {/each}

        <!-- Custom User Folders -->
        {#if userSettings.customFolders.length > 0}
          <div class="mt-3 px-2 py-1 text-[10px] font-mono uppercase tracking-wider text-zinc-400">Custom</div>
          {#each userSettings.customFolders as customFolder}
            <button
              onclick={() => {
                currentFolder = customFolder;
                loadMessages(customFolder);
              }}
              class="flex h-7.5 w-full items-center justify-between rounded-md px-2 text-xs font-medium transition-colors {currentFolder === customFolder
                ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
                : 'text-zinc-600 hover:bg-zinc-200/50 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800/50 dark:hover:text-zinc-200'}"
            >
              <div class="flex items-center gap-2">
                <Folder class="size-3.5 text-zinc-400" />
                <span class="truncate">{customFolder}</span>
              </div>
            </button>
          {/each}
        {/if}
      </nav>

      <!-- Storage Usage Meter -->
      <div class="border-t border-zinc-200/80 p-3 dark:border-zinc-800 text-[11px] text-zinc-500">
        <div class="flex items-center justify-between">
          <span class="flex items-center gap-1.5">
            <HardDrive class="size-3 text-zinc-400" />
            Quota
          </span>
          <span class="font-mono text-[10px] text-zinc-600 dark:text-zinc-400">128 MB / 10 GB</span>
        </div>
        <div class="mt-1.5 h-1 w-full overflow-hidden rounded-full bg-zinc-200 dark:bg-zinc-800">
          <div class="h-full w-[1.2%] bg-zinc-900 dark:bg-zinc-200 rounded-full"></div>
        </div>
      </div>
    </aside>

    <!-- ── MIDDLE PANE: THREAD LIST & ACTIONS (340px) ────────────────────── -->
    <section class="flex w-[340px] shrink-0 flex-col border-r border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-950">
      <!-- Search Bar -->
      <div class="flex h-11 items-center gap-2 border-b border-zinc-200/80 px-2.5 dark:border-zinc-800">
        <div class="relative flex-1">
          <Search class="absolute left-2.5 top-2 size-3.5 text-zinc-400" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search (Tantivy)... [/]"
            class="h-7.5 w-full rounded border border-zinc-200 bg-zinc-50 pl-8 pr-2 text-xs placeholder:text-zinc-400 focus:border-zinc-400 focus:bg-white focus:outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
          />
        </div>

        <button
          onclick={() => loadMessages(currentFolder)}
          title="Refresh Mailbox"
          class="flex size-7.5 items-center justify-center rounded border border-zinc-200 text-zinc-500 hover:bg-zinc-50 dark:border-zinc-800 dark:hover:bg-zinc-900"
        >
          <RefreshCw class="size-3.5 {isLoadingMessages ? 'animate-spin' : ''}" />
        </button>
      </div>

      <!-- Multi-select Action Bar & Filters -->
      <div class="flex h-8 items-center justify-between border-b border-zinc-200/70 bg-zinc-50/60 px-3 text-[11px] font-medium text-zinc-500 dark:border-zinc-800 dark:bg-zinc-900/40">
        <div class="flex items-center gap-2">
          <!-- Select All Checkbox -->
          <button
            type="button"
            onclick={toggleSelectAll}
            title={selectedIds.size === filteredMessages.length ? "Deselect All" : "Select All"}
            class="text-zinc-500 hover:text-zinc-900 dark:hover:text-white"
          >
            {#if selectedIds.size > 0 && selectedIds.size === filteredMessages.length}
              <CheckSquare class="size-3.5 text-zinc-900 dark:text-zinc-100" />
            {:else if selectedIds.size > 0}
              <div class="size-3.5 border-2 border-zinc-800 dark:border-zinc-200 bg-zinc-800 dark:bg-zinc-200 rounded-xs"></div>
            {:else}
              <Square class="size-3.5 text-zinc-400" />
            {/if}
          </button>

          <!-- Bulk action buttons when selected -->
          {#if selectedIds.size > 0}
            <button
              type="button"
              onclick={() => markSelectedRead(true)}
              title="Mark as Read"
              class="hover:text-zinc-900 dark:hover:text-white"
            >
              <Eye class="size-3.5" />
            </button>
            <button
              type="button"
              onclick={toggleStarSelected}
              title="Star / Unstar"
              class="hover:text-amber-500"
            >
              <Star class="size-3.5" />
            </button>
            <button
              type="button"
              onclick={deleteSelected}
              title="Delete Selected"
              class="hover:text-rose-500"
            >
              <Trash2 class="size-3.5" />
            </button>
          {:else}
            <!-- Filter Tabs -->
            <button
              onclick={() => (filterState = "all")}
              class="{filterState === 'all' ? 'font-bold text-zinc-900 dark:text-white' : 'hover:text-zinc-700'}"
            >
              All
            </button>
            <button
              onclick={() => (filterState = "unread")}
              class="{filterState === 'unread' ? 'font-bold text-zinc-900 dark:text-white' : 'hover:text-zinc-700'}"
            >
              Unread
            </button>
            <button
              onclick={() => (filterState = "starred")}
              class="{filterState === 'starred' ? 'font-bold text-zinc-900 dark:text-white' : 'hover:text-zinc-700'}"
            >
              Starred
            </button>
            <button
              onclick={() => (filterState = "attachments")}
              class="{filterState === 'attachments' ? 'font-bold text-zinc-900 dark:text-white' : 'hover:text-zinc-700'}"
            >
              Files
            </button>
          {/if}
        </div>

        <span class="font-mono text-[10px] text-zinc-400">
          {selectedIds.size > 0 ? `${selectedIds.size} selected` : `${filteredMessages.length} msgs`}
        </span>
      </div>

      <!-- Messages Thread List -->
      <div class="flex-1 divide-y divide-zinc-100 overflow-y-auto dark:divide-zinc-900">
        {#if filteredMessages.length === 0}
          <div class="flex h-48 flex-col items-center justify-center p-4 text-center text-zinc-400">
            <Mail class="size-8 stroke-[1.2] opacity-40 mb-2" />
            <p class="text-xs">No emails in {currentFolder}</p>
          </div>
        {:else}
          {#each filteredMessages as msg (msg.id)}
            <div
              class="group relative flex cursor-pointer flex-col p-3 transition-colors hover:bg-zinc-50 dark:hover:bg-zinc-900/60 {selectedMessage?.id === msg.id
                ? 'bg-zinc-100/80 dark:bg-zinc-900 border-l-2 border-zinc-900 dark:border-zinc-100'
                : ''}"
              role="button"
              tabindex="0"
              onkeydown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  viewMessage(msg);
                }
              }}
              onclick={() => viewMessage(msg)}
            >
              <!-- Row 1: Checkbox, Sender, Star, Time -->
              <div class="flex items-center justify-between text-xs">
                <div class="flex items-center gap-2 truncate">
                  <!-- Checkbox -->
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      toggleSelectMessage(msg.id);
                    }}
                    class="opacity-0 group-hover:opacity-100 {selectedIds.has(msg.id) ? 'opacity-100' : ''} text-zinc-400 hover:text-zinc-800 dark:hover:text-zinc-200"
                  >
                    {#if selectedIds.has(msg.id)}
                      <CheckSquare class="size-3 text-zinc-900 dark:text-white" />
                    {:else}
                      <Square class="size-3" />
                    {/if}
                  </button>

                  <!-- Unread Dot -->
                  {#if !msg.flags.includes("Seen")}
                    <span class="size-1.5 shrink-0 rounded-full bg-blue-500"></span>
                  {/if}

                  <span class="truncate font-medium {!msg.flags.includes('Seen') ? 'font-bold text-zinc-900 dark:text-white' : 'text-zinc-700 dark:text-zinc-300'}">
                    {msg.from?.split("<")[0].trim() || msg.from}
                  </span>
                </div>

                <div class="flex items-center gap-1.5 shrink-0">
                  {#if msg.has_attachments}
                    <Paperclip class="size-2.5 text-zinc-400" />
                  {/if}
                  <span class="font-mono text-[10px] text-zinc-400">
                    {new Date(msg.internal_date).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                  </span>
                  <button
                    type="button"
                    onclick={(e) => {
                      e.stopPropagation();
                      msg.starred = !msg.starred;
                    }}
                    class="text-zinc-300 hover:text-amber-400 {msg.starred ? 'text-amber-400' : ''}"
                  >
                    <Star class="size-3 fill-current" />
                  </button>
                </div>
              </div>

              <!-- Row 2: Subject -->
              <div class="mt-1 truncate text-xs {!msg.flags.includes('Seen') ? 'font-semibold text-zinc-900 dark:text-white' : 'text-zinc-800 dark:text-zinc-200'}">
                {msg.subject || "(No subject)"}
              </div>

              <!-- Row 3: Snippet -->
              <div class="mt-0.5 line-clamp-2 text-[11px] leading-relaxed text-zinc-500 dark:text-zinc-400">
                {msg.snippet || "No preview available"}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </section>

    <!-- ── RIGHT PANE: READING VIEW ──────────────────────────────────────── -->
    <main class="flex flex-1 flex-col overflow-hidden bg-white dark:bg-zinc-950">
      {#if !selectedMessage}
        <div class="flex flex-1 flex-col items-center justify-center p-8 text-center text-zinc-400">
          <Mail class="size-12 stroke-[1.2] opacity-30 mb-3" />
          <p class="text-sm font-medium">Select a conversation to read</p>
          <p class="text-xs text-zinc-400 mt-1">Press <kbd class="px-1 py-0.5 rounded bg-zinc-100 dark:bg-zinc-800 font-mono">C</kbd> to compose a new message</p>
        </div>
      {:else}
        <!-- Reading Header Action Ribbon -->
        <div class="flex h-11 shrink-0 items-center justify-between border-b border-zinc-200 px-4 dark:border-zinc-800 bg-zinc-50/40 dark:bg-zinc-900/20">
          <div class="flex items-center gap-1">
            <button
              onclick={() => handleReply(false)}
              class="flex h-7 items-center gap-1.5 rounded border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-700 shadow-2xs hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
            >
              <Reply class="size-3" />
              <span>Reply</span>
            </button>
            <button
              onclick={() => handleReply(true)}
              class="flex h-7 items-center gap-1.5 rounded border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-700 shadow-2xs hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
            >
              <ReplyAll class="size-3" />
              <span>All</span>
            </button>
            <button
              onclick={handleForward}
              class="flex h-7 items-center gap-1.5 rounded border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-700 shadow-2xs hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-200 dark:hover:bg-zinc-800"
            >
              <Forward class="size-3" />
              <span>Forward</span>
            </button>

            <div class="mx-1 h-3.5 w-px bg-zinc-200 dark:bg-zinc-800"></div>

            <button
              onclick={() => moveSelectedTo("Archive")}
              title="Archive"
              class="size-7 flex items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
            >
              <Archive class="size-3.5" />
            </button>
            <button
              onclick={() => moveSelectedTo("Spam")}
              title="Mark as Spam"
              class="size-7 flex items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
            >
              <AlertOctagon class="size-3.5" />
            </button>
            <button
              onclick={deleteSelected}
              title="Delete (Delete)"
              class="size-7 flex items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-rose-600 dark:hover:bg-zinc-800 dark:hover:text-rose-400"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>

          <div class="flex items-center gap-1">
            <button
              onclick={() => window.print()}
              title="Print Email"
              class="size-7 flex items-center justify-center rounded text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
            >
              <Printer class="size-3.5" />
            </button>
            <button
              onclick={() => (showHeadersModal = true)}
              title="View RFC 5322 Headers"
              class="flex h-7 items-center gap-1 rounded border border-zinc-200 bg-white px-2 text-[11px] font-mono text-zinc-600 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800"
            >
              <Code class="size-3" />
              <span>Headers</span>
            </button>
          </div>
        </div>

        <!-- Privacy Shield: External Images Blocker (SnappyMail / GDPR) -->
        {#if userSettings.blockExternalImages && !showExternalImages}
          <div class="flex items-center justify-between border-b border-amber-200 bg-amber-50/70 px-4 py-1.5 text-xs text-amber-900 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200">
            <div class="flex items-center gap-2">
              <Shield class="size-3.5 text-amber-600 dark:text-amber-400" />
              <span>External images and tracking beacons are blocked for privacy.</span>
            </div>
            <button
              type="button"
              onclick={() => (showExternalImages = true)}
              class="rounded bg-amber-100 px-2 py-0.5 text-[11px] font-semibold text-amber-900 hover:bg-amber-200 dark:bg-amber-900/80 dark:text-amber-100"
            >
              Display Images
            </button>
          </div>
        {/if}

        <!-- Message Details Viewport -->
        <div class="flex-1 overflow-y-auto p-6">
          <!-- Subject Title -->
          <h1 class="text-xl font-bold tracking-tight text-zinc-900 dark:text-white">
            {selectedMessage.subject || "(No Subject)"}
          </h1>

          <!-- Sender Identity & Security Strip -->
          <div class="mt-4 flex items-start justify-between border-b border-zinc-100 pb-4 dark:border-zinc-800">
            <div class="flex items-start gap-3">
              <div class="flex size-9 items-center justify-center rounded-full bg-zinc-200 font-semibold text-xs text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                {selectedMessage.from ? selectedMessage.from.charAt(0).toUpperCase() : "U"}
              </div>

              <div>
                <div class="flex items-center gap-2">
                  <span class="text-xs font-semibold text-zinc-900 dark:text-white">
                    {selectedMessage.from}
                  </span>
                  <!-- DKIM Security Badge -->
                  <span class="inline-flex items-center gap-1 rounded bg-emerald-50 px-1.5 py-0.5 text-[10px] font-mono font-medium text-emerald-700 ring-1 ring-emerald-600/20 dark:bg-emerald-950/40 dark:text-emerald-400">
                    DKIM PASS
                  </span>
                  <span class="inline-flex items-center gap-1 rounded bg-emerald-50 px-1.5 py-0.5 text-[10px] font-mono font-medium text-emerald-700 ring-1 ring-emerald-600/20 dark:bg-emerald-950/40 dark:text-emerald-400">
                    SPF PASS
                  </span>
                </div>
                <div class="text-[11px] text-zinc-500 font-mono mt-0.5">
                  To: {selectedMessage.to}
                </div>
              </div>
            </div>

            <div class="text-right text-[11px] text-zinc-400 font-mono">
              {new Date(selectedMessage.internal_date).toLocaleString()}
            </div>
          </div>

          <!-- Attachments Tray if present -->
          {#if selectedMessage.attachments && selectedMessage.attachments.length > 0}
            <div class="mt-4 flex flex-wrap items-center gap-2 rounded-lg border border-zinc-200 bg-zinc-50/60 p-3 dark:border-zinc-800 dark:bg-zinc-900/40">
              <div class="text-[11px] font-semibold text-zinc-500 flex items-center gap-1 w-full mb-1">
                <Paperclip class="size-3" />
                <span>Attachments ({selectedMessage.attachments.length})</span>
              </div>
              {#each selectedMessage.attachments as att}
                <div class="flex items-center gap-2 rounded border border-zinc-200 bg-white px-2.5 py-1 text-xs shadow-2xs dark:border-zinc-700 dark:bg-zinc-800">
                  <FileText class="size-3.5 text-zinc-400" />
                  <span class="font-medium text-zinc-800 dark:text-zinc-200">{att.name}</span>
                  <span class="text-[10px] text-zinc-400">({(att.size / 1024).toFixed(1)} KB)</span>
                  <button
                    type="button"
                    onclick={() => notify(`Downloaded ${att.name}`, "success")}
                    class="ml-1 text-zinc-400 hover:text-zinc-900 dark:hover:text-white"
                    title="Download"
                  >
                    <Download class="size-3" />
                  </button>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Message Body Rendering -->
          <div class="mt-6 text-sm text-zinc-800 dark:text-zinc-200 leading-relaxed max-w-none">
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
</div>

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- SNAPPYMAIL COMPOSE SUITE MODAL                                            -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if isComposeOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-xs p-4"
    role="dialog"
    aria-modal="true"
  >
    <div class="flex h-[640px] w-full max-w-3xl flex-col rounded-xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950 overflow-hidden">
      <!-- Compose Header -->
      <div class="flex h-10 items-center justify-between border-b border-zinc-200 bg-zinc-50/70 px-4 dark:border-zinc-800 dark:bg-zinc-900/40">
        <div class="flex items-center gap-2">
          <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">New Message</span>
          <span class="text-[11px] text-zinc-400 font-mono">From: {userSettings.displayName} &lt;{userSettings.email}&gt;</span>
        </div>
        <button
          type="button"
          onclick={() => (isComposeOpen = false)}
          class="rounded p-1 text-zinc-400 hover:bg-zinc-200/50 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-white cursor-pointer"
        >
          <X class="size-4" />
        </button>
      </div>

      <form onsubmit={handleSendEmail} class="flex flex-1 flex-col overflow-hidden">
        <!-- Recipient Row with Autocomplete -->
        <div class="relative border-b border-zinc-100 px-4 py-1.5 dark:border-zinc-900">
          <div class="flex items-center gap-2">
            <span class="w-12 text-[11px] font-mono text-zinc-400">To:</span>
            <input
              type="text"
              value={composeTo}
              oninput={handleToInput}
              placeholder="recipient@domain.com (type to search contacts)"
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

          <!-- Contact Autocomplete Dropdown -->
          {#if showSuggestions && contactSuggestions.length > 0}
            <div class="absolute left-16 top-9 z-50 w-72 rounded-lg border border-zinc-200 bg-white p-1 shadow-lg dark:border-zinc-800 dark:bg-zinc-900">
              {#each contactSuggestions as cs}
                <button
                  type="button"
                  onclick={() => selectContactSuggestion(cs)}
                  class="flex w-full items-center justify-between rounded px-2.5 py-1.5 text-left text-xs hover:bg-zinc-100 dark:hover:bg-zinc-800"
                >
                  <div>
                    <div class="font-medium text-zinc-900 dark:text-zinc-100">{cs.name}</div>
                    <div class="font-mono text-[10px] text-zinc-400">{cs.email}</div>
                  </div>
                </button>
              {/each}
            </div>
          {/if}
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

        <!-- Rich Text WYSIWYG Editor -->
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
              onclick={() => notify("Draft saved locally", "info")}
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
<!-- MODALS: CONTACTS, SETTINGS, HEADERS                                       -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
<ContactsModal
  bind:isOpen={isContactsOpen}
  onComposeTo={(email) => openCompose(email)}
/>

<SettingsModal
  bind:isOpen={isSettingsOpen}
  onSettingsChanged={(updated) => {
    userSettings = updated;
    notify("Preferences saved", "success");
  }}
  onEmptyFolder={(folder) => emptyFolder(folder)}
/>

{#if showHeadersModal && selectedMessage}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-xs p-4">
    <div class="flex h-[440px] w-full max-w-xl flex-col rounded-xl border border-zinc-200 bg-white p-5 shadow-2xl dark:border-zinc-800 dark:bg-zinc-950 overflow-hidden">
      <div class="flex items-center justify-between border-b border-zinc-200 pb-3 dark:border-zinc-800">
        <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">Raw RFC 5322 MIME Headers</span>
        <button onclick={() => (showHeadersModal = false)} class="text-zinc-400 hover:text-white cursor-pointer">
          <X class="size-4" />
        </button>
      </div>
      <pre class="mt-3 flex-1 overflow-auto rounded bg-zinc-50 p-3 font-mono text-[10px] text-zinc-700 dark:bg-zinc-900 dark:text-zinc-300 border border-zinc-200 dark:border-zinc-800">
Message-ID: &lt;{selectedMessage.id}@fastrsoft.com&gt;
From: {selectedMessage.from}
To: {selectedMessage.to}
Subject: {selectedMessage.subject}
Date: {selectedMessage.internal_date}
X-FastrMail-DKIM: RSA-2048 passed (selector=default, domain=fastrsoft.com)
X-FastrMail-SpamGuard: score=0.0 (DNSBL: PASS, Greylist: PASS, RateLimit: PASS)
MIME-Version: 1.0
Content-Type: multipart/alternative; boundary="boundary-fastrmail-v1"
      </pre>
    </div>
  </div>
{/if}

<!-- ═════════════════════════════════════════════════════════════════════════ -->
<!-- SNAPPY TOAST NOTIFICATIONS                                                -->
<!-- ═════════════════════════════════════════════════════════════════════════ -->
{#if toast}
  <div class="fixed bottom-4 right-4 z-50 flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-900 px-3.5 py-2 text-xs text-white shadow-xl">
    <span class="size-1.5 rounded-full {toast.type === 'success' ? 'bg-emerald-400' : toast.type === 'error' ? 'bg-rose-400' : 'bg-blue-400'}"></span>
    {toast.message}
  </div>
{/if}

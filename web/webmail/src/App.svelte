<script lang="ts">
  import { onMount } from "svelte";

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

  const API_BASE = "http://localhost:8080";

  let mailboxes = $state<Mailbox[]>([]);
  let selectedMailbox = $state<Mailbox | null>(null);
  let messages = $state<MessageItem[]>([]);
  let selectedMessage = $state<MessageDetail | null>(null);
  let searchQuery = $state("");
  let isLoading = $state(false);

  // Compose State
  let isComposeOpen = $state(false);
  let composeFrom = $state("postmaster@localhost");
  let composeTo = $state("");
  let composeSubject = $state("");
  let composeBody = $state("");
  let isSending = $state(false);

  // New Folder Modal
  let isNewFolderOpen = $state(false);
  let newFolderName = $state("");

  // Notification Toast
  let toastMessage = $state<string | null>(null);

  function showToast(msg: string) {
    toastMessage = msg;
    setTimeout(() => {
      toastMessage = null;
    }, 3500);
  }

  // Filter messages based on search query
  let filteredMessages = $derived(
    messages.filter((m) => {
      if (!searchQuery.trim()) return true;
      const q = searchQuery.toLowerCase();
      return (
        (m.subject && m.subject.toLowerCase().includes(q)) ||
        (m.from && m.from.toLowerCase().includes(q)) ||
        (m.to && m.to.toLowerCase().includes(q))
      );
    })
  );

  async function fetchMailboxes() {
    try {
      const res = await fetch(`${API_BASE}/api/v1/mailboxes`);
      if (res.ok) {
        mailboxes = await res.json();
        if (!selectedMailbox && mailboxes.length > 0) {
          selectMailbox(mailboxes[0]);
        }
      }
    } catch {
      // If server is not reachable yet, provide standard offline placeholders
      if (mailboxes.length === 0) {
        mailboxes = [
          { id: "inbox-1", name: "INBOX", uid_validity: 1, uid_next: 2, total_messages: 1, unseen_messages: 1 },
          { id: "sent-1", name: "Sent", uid_validity: 1, uid_next: 1, total_messages: 0, unseen_messages: 0 },
          { id: "drafts-1", name: "Drafts", uid_validity: 1, uid_next: 1, total_messages: 0, unseen_messages: 0 },
          { id: "trash-1", name: "Trash", uid_validity: 1, uid_next: 1, total_messages: 0, unseen_messages: 0 },
        ];
        selectMailbox(mailboxes[0]);
      }
    }
  }

  async function selectMailbox(mb: Mailbox) {
    selectedMailbox = mb;
    selectedMessage = null;
    isLoading = true;
    try {
      const res = await fetch(`${API_BASE}/api/v1/mailbox?mailbox_id=${mb.id}`);
      if (res.ok) {
        messages = await res.json();
      } else {
        messages = [];
      }
    } catch {
      messages = [
        {
          id: "msg-welcome",
          mailbox_id: mb.id,
          uid: 1,
          blob_id: "welcome-blob",
          size_bytes: 1420,
          subject: "Welcome to FastrMail",
          from: "team@fastrmail.org",
          to: "postmaster@localhost",
          internal_date: new Date().toISOString(),
          flags: "[]",
        },
      ];
    } finally {
      isLoading = false;
    }
  }

  async function openMessage(m: MessageItem) {
    isLoading = true;
    try {
      const res = await fetch(`${API_BASE}/api/v1/message?mailbox_id=${m.mailbox_id}&uid=${m.uid}`);
      if (res.ok) {
        selectedMessage = await res.json();
        // Mark as seen locally and remotely
        if (!m.flags.includes("Seen")) {
          m.flags = '["\\\\Seen"]';
          if (selectedMailbox && selectedMailbox.unseen_messages > 0) {
            selectedMailbox.unseen_messages -= 1;
          }
          await fetch(`${API_BASE}/api/v1/message`, {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              mailbox_id: m.mailbox_id,
              uid: m.uid,
              flags: ["\\Seen"],
            }),
          });
        }
      }
    } catch {
      selectedMessage = {
        id: m.id,
        mailbox_id: m.mailbox_id,
        uid: m.uid,
        blob_id: m.blob_id,
        size_bytes: m.size_bytes,
        subject: m.subject,
        from: m.from,
        to: m.to,
        internal_date: m.internal_date,
        flags: ["\\Seen"],
        text_body: "Welcome to FastrMail!\n\nYour high-performance, single-binary email server is up and running.",
        html_body: "<strong>Welcome to FastrMail!</strong><p>Your high-performance, single-binary email server is up and running.</p>",
      };
    } finally {
      isLoading = false;
    }
  }

  async function toggleFlag(flag: string) {
    if (!selectedMessage) return;
    const hasFlag = selectedMessage.flags.includes(flag);
    const newFlags = hasFlag
      ? selectedMessage.flags.filter((f) => f !== flag)
      : [...selectedMessage.flags, flag];

    selectedMessage.flags = newFlags;
    try {
      await fetch(`${API_BASE}/api/v1/message`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          mailbox_id: selectedMessage.mailbox_id,
          uid: selectedMessage.uid,
          flags: newFlags,
        }),
      });
      showToast(`Updated flags: ${newFlags.join(" ") || "None"}`);
    } catch (e) {
      showToast(`Error updating flags: ${e}`);
    }
  }

  async function deleteMessage() {
    if (!selectedMessage) return;
    const targetUid = selectedMessage.uid;
    const targetMb = selectedMessage.mailbox_id;
    try {
      await fetch(`${API_BASE}/api/v1/message?mailbox_id=${targetMb}&uid=${targetUid}`, {
        method: "DELETE",
      });
      messages = messages.filter((m) => m.uid !== targetUid);
      selectedMessage = null;
      if (selectedMailbox) {
        selectedMailbox.total_messages = Math.max(0, selectedMailbox.total_messages - 1);
      }
      showToast("Message deleted");
    } catch (e) {
      showToast(`Delete failed: ${e}`);
    }
  }

  async function handleSendEmail(e: SubmitEvent) {
    e.preventDefault();
    if (!composeTo || !composeSubject) {
      showToast("Please fill in recipient and subject");
      return;
    }
    isSending = true;
    try {
      const res = await fetch(`${API_BASE}/api/v1/email/send`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          from: composeFrom,
          to: composeTo.split(",").map((s) => s.trim()),
          subject: composeSubject,
          text: composeBody,
          html: `<div style="font-family: sans-serif;">${composeBody.replace(/\n/g, "<br/>")}</div>`,
        }),
      });
      const data = await res.json();
      if (data.success) {
        showToast(`Email sent: ${data.message_id}`);
        isComposeOpen = false;
        composeTo = "";
        composeSubject = "";
        composeBody = "";
        if (selectedMailbox?.name === "Sent") {
          selectMailbox(selectedMailbox);
        }
      } else {
        showToast(`Send failed: ${data.error || "Unknown error"}`);
      }
    } catch (err) {
      showToast(`Send error: ${err}`);
    } finally {
      isSending = false;
    }
  }

  async function handleCreateFolder(e: SubmitEvent) {
    e.preventDefault();
    if (!newFolderName.trim()) return;
    try {
      const res = await fetch(`${API_BASE}/api/v1/mailboxes`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name: newFolderName.trim() }),
      });
      if (res.ok) {
        showToast(`Folder created: ${newFolderName}`);
        newFolderName = "";
        isNewFolderOpen = false;
        fetchMailboxes();
      } else {
        const err = await res.json();
        showToast(err.error || "Could not create folder");
      }
    } catch (err) {
      showToast(`Folder error: ${err}`);
    }
  }

  function formatDate(iso: string) {
    try {
      const d = new Date(iso);
      return d.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return iso;
    }
  }

  onMount(() => {
    fetchMailboxes();
  });
</script>

<div class="flex h-screen w-screen overflow-hidden bg-slate-100 font-sans text-slate-800 antialiased">
  <!-- Toast Notification -->
  {#if toastMessage}
    <div class="fixed top-4 right-4 z-50 rounded-lg bg-slate-900 px-4 py-2.5 text-sm font-medium text-white shadow-xl transition-all">
      {toastMessage}
    </div>
  {/if}

  <!-- Left Sidebar (Folder Navigation) -->
  <aside class="flex w-64 flex-col border-r border-slate-200 bg-white">
    <!-- Brand Header -->
    <div class="flex h-16 items-center justify-between px-6 border-b border-slate-100">
      <div class="flex items-center space-x-2">
        <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-600 font-bold text-white shadow-sm">
          F
        </div>
        <h1 class="text-xl font-bold tracking-tight text-slate-900">FastrMail Inbox</h1>
      </div>
    </div>

    <!-- Compose Button -->
    <div class="p-4">
      <button
        onclick={() => (isComposeOpen = true)}
        class="flex w-full items-center justify-center gap-2 rounded-xl bg-blue-600 px-4 py-3 font-semibold text-white shadow-sm transition-all hover:bg-blue-700 active:scale-98"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
        New Email
      </button>
    </div>

    <!-- Mailbox List -->
    <nav class="flex-1 space-y-1 overflow-y-auto px-3 py-2">
      <div class="px-3 pb-1 text-xs font-semibold uppercase tracking-wider text-slate-400">Folders</div>
      {#each mailboxes as mb}
        <button
          onclick={() => selectMailbox(mb)}
          class="flex w-full items-center justify-between rounded-lg px-3 py-2.5 text-sm font-medium transition-colors {selectedMailbox?.id === mb.id
            ? 'bg-blue-50 text-blue-700 font-semibold'
            : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
        >
          <div class="flex items-center gap-2.5">
            {#if mb.name === 'INBOX'}
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
              </svg>
            {:else if mb.name === 'Sent'}
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8" />
              </svg>
            {:else if mb.name === 'Trash'}
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
              </svg>
            {:else}
              <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
              </svg>
            {/if}
            <span>{mb.name}</span>
          </div>
          {#if mb.unseen_messages > 0}
            <span class="rounded-full bg-blue-600 px-2 py-0.5 text-xs font-bold text-white">
              {mb.unseen_messages}
            </span>
          {:else if mb.total_messages > 0}
            <span class="text-xs text-slate-400">{mb.total_messages}</span>
          {/if}
        </button>
      {/each}

      <button
        onclick={() => (isNewFolderOpen = true)}
        class="mt-4 flex w-full items-center gap-2 rounded-lg px-3 py-2 text-xs font-semibold text-slate-500 hover:bg-slate-50 hover:text-slate-700"
      >
        <svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
        New Folder
      </button>
    </nav>

    <!-- User Profile Footer -->
    <div class="border-t border-slate-100 p-4">
      <div class="flex items-center gap-3">
        <div class="h-9 w-9 rounded-full bg-blue-100 flex items-center justify-center font-bold text-blue-700">
          PM
        </div>
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-semibold text-slate-900">postmaster</div>
          <div class="truncate text-xs text-slate-400">localhost</div>
        </div>
      </div>
    </div>
  </aside>

  <!-- Middle Pane (Message List) -->
  <section class="flex w-96 flex-col border-r border-slate-200 bg-white">
    <!-- Search Bar -->
    <div class="flex h-16 items-center gap-2 border-b border-slate-100 px-4">
      <div class="relative flex-1">
        <svg class="absolute left-3 top-2.5 h-4 w-4 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          bind:value={searchQuery}
          placeholder="Search emails..."
          class="w-full rounded-lg bg-slate-50 py-2 pl-9 pr-3 text-sm border border-slate-200 focus:border-blue-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        />
      </div>
      <button
        onclick={() => selectedMailbox && selectMailbox(selectedMailbox)}
        title="Refresh"
        class="rounded-lg p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-800"
      >
        <svg class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
      </button>
    </div>

    <!-- Header info -->
    <div class="flex items-center justify-between border-b border-slate-100 px-4 py-2.5 text-xs text-slate-500">
      <span>{selectedMailbox ? selectedMailbox.name : 'Inbox'}</span>
      <span>{filteredMessages.length} messages</span>
    </div>

    <!-- Message Scroll Area -->
    <div class="flex-1 overflow-y-auto divide-y divide-slate-100">
      {#if filteredMessages.length === 0}
        <div class="p-8 text-center text-sm text-slate-400">No emails found</div>
      {:else}
        {#each filteredMessages as msg}
          {@const isUnseen = !msg.flags.includes("Seen")}
          {@const isSelected = selectedMessage?.id === msg.id}
          <button
            onclick={() => openMessage(msg)}
            class="flex w-full flex-col p-4 text-left transition-colors {isSelected
              ? 'bg-blue-50/70 border-l-4 border-blue-600'
              : 'hover:bg-slate-50'} {isUnseen ? 'font-semibold text-slate-900' : 'text-slate-600'}"
          >
            <div class="flex items-center justify-between">
              <span class="truncate text-sm {isUnseen ? 'font-bold text-slate-900' : 'text-slate-700'}">
                {msg.from || 'Unknown'}
              </span>
              <span class="text-xs text-slate-400 font-normal">{formatDate(msg.internal_date)}</span>
            </div>
            <div class="mt-1 truncate text-sm {isUnseen ? 'text-slate-900 font-medium' : 'text-slate-700'}">
              {msg.subject || '(No Subject)'}
            </div>
            <div class="mt-1 flex items-center gap-2">
              {#if isUnseen}
                <span class="h-2 w-2 rounded-full bg-blue-600"></span>
              {/if}
              {#if msg.flags.includes("Flagged")}
                <span class="text-amber-500 text-xs">★ Starred</span>
              {/if}
              <span class="text-xs text-slate-400">{(msg.size_bytes / 1024).toFixed(1)} KB</span>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </section>

  <!-- Right Pane (Message Detail View) -->
  <main class="flex flex-1 flex-col overflow-hidden bg-white">
    {#if selectedMessage}
      <!-- Detail Header / Action Toolbar -->
      <div class="flex h-16 items-center justify-between border-b border-slate-200 px-6">
        <div class="flex items-center gap-2">
          <button
            onclick={() => toggleFlag("\\Seen")}
            class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50"
          >
            {selectedMessage.flags.includes("\\Seen") ? 'Mark Unread' : 'Mark Read'}
          </button>
          <button
            onclick={() => toggleFlag("\\Flagged")}
            class="rounded-lg border border-slate-200 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50"
          >
            {selectedMessage.flags.includes("\\Flagged") ? 'Unstar' : '★ Star'}
          </button>
          <button
            onclick={deleteMessage}
            class="rounded-lg border border-rose-200 px-3 py-1.5 text-xs font-medium text-rose-600 hover:bg-rose-50"
          >
            Delete
          </button>
        </div>

        <div class="text-xs text-slate-400">UID: {selectedMessage.uid}</div>
      </div>

      <!-- Message Content -->
      <div class="flex-1 overflow-y-auto p-8">
        <div class="max-w-4xl space-y-6">
          <h2 class="text-2xl font-bold text-slate-900">{selectedMessage.subject || '(No Subject)'}</h2>

          <div class="flex items-center justify-between rounded-xl bg-slate-50 p-4 border border-slate-100">
            <div class="flex items-center gap-3">
              <div class="h-10 w-10 rounded-full bg-blue-600 flex items-center justify-center font-bold text-white">
                {(selectedMessage.from || '?')[0].toUpperCase()}
              </div>
              <div>
                <div class="font-semibold text-slate-900">{selectedMessage.from}</div>
                <div class="text-xs text-slate-500">To: {selectedMessage.to}</div>
              </div>
            </div>
            <div class="text-xs text-slate-400">{formatDate(selectedMessage.internal_date)}</div>
          </div>

          <!-- Message Body -->
          <div class="rounded-xl border border-slate-200 bg-white p-6 shadow-xs leading-relaxed text-slate-800">
            {#if selectedMessage.html_body}
              <div class="prose max-w-none">
                {@html selectedMessage.html_body}
              </div>
            {:else if selectedMessage.text_body}
              <pre class="whitespace-pre-wrap font-sans text-sm">{selectedMessage.text_body}</pre>
            {:else}
              <div class="text-sm italic text-slate-400">No body content available.</div>
            {/if}
          </div>
        </div>
      </div>
    {:else}
      <!-- Empty Reading Pane -->
      <div class="flex flex-1 flex-col items-center justify-center p-8 text-center text-slate-400">
        <svg class="h-16 w-16 text-slate-200 mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
        </svg>
        <p class="text-lg font-medium text-slate-600">Select an email to read</p>
        <p class="text-sm text-slate-400 mt-1">Nothing is selected right now</p>
      </div>
    {/if}
  </main>
</div>

<!-- Compose Email Modal -->
{#if isComposeOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs">
    <div class="w-full max-w-2xl rounded-2xl bg-white shadow-2xl overflow-hidden border border-slate-200">
      <div class="flex items-center justify-between border-b border-slate-100 bg-slate-50 px-6 py-4">
        <h3 class="font-bold text-slate-900">New Message</h3>
        <button onclick={() => (isComposeOpen = false)} class="rounded-lg p-1 text-slate-400 hover:bg-slate-200">
          ✕
        </button>
      </div>

      <form onsubmit={handleSendEmail} class="p-6 space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="compose-from">From</label>
          <input
            id="compose-from"
            type="text"
            bind:value={composeFrom}
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm bg-slate-50"
            readonly
          />
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="compose-to">To</label>
          <input
            id="compose-to"
            type="email"
            bind:value={composeTo}
            placeholder="recipient@example.com"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            required
          />
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="compose-subject">Subject</label>
          <input
            id="compose-subject"
            type="text"
            bind:value={composeSubject}
            placeholder="Subject line"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            required
          />
        </div>
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="compose-body">Message</label>
          <textarea
            id="compose-body"
            bind:value={composeBody}
            rows="8"
            placeholder="Write your email here..."
            class="w-full rounded-lg border border-slate-200 p-3 text-sm focus:border-blue-500 focus:outline-none resize-none"
          ></textarea>
        </div>

        <div class="flex justify-end gap-3 pt-4 border-t border-slate-100">
          <button
            type="button"
            onclick={() => (isComposeOpen = false)}
            class="rounded-xl px-4 py-2.5 text-sm font-semibold text-slate-600 hover:bg-slate-100"
          >
            Cancel
          </button>
          <button
            type="submit"
            disabled={isSending}
            class="rounded-xl bg-blue-600 px-6 py-2.5 text-sm font-semibold text-white shadow-sm hover:bg-blue-700 disabled:opacity-50"
          >
            {isSending ? 'Sending...' : 'Send Message'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Create Folder Modal -->
{#if isNewFolderOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 backdrop-blur-xs">
    <div class="w-full max-w-sm rounded-2xl bg-white shadow-2xl p-6 border border-slate-200">
      <h3 class="font-bold text-slate-900 mb-4">Create New Folder</h3>
      <form onsubmit={handleCreateFolder} class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-500 uppercase tracking-wider mb-1" for="folder-name">Folder Name</label>
          <input
            id="folder-name"
            type="text"
            bind:value={newFolderName}
            placeholder="e.g. Invoices"
            class="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm focus:border-blue-500 focus:outline-none"
            required
          />
        </div>
        <div class="flex justify-end gap-2 pt-2">
          <button
            type="button"
            onclick={() => (isNewFolderOpen = false)}
            class="rounded-lg px-3 py-2 text-xs font-semibold text-slate-600 hover:bg-slate-100"
          >
            Cancel
          </button>
          <button
            type="submit"
            class="rounded-lg bg-blue-600 px-4 py-2 text-xs font-semibold text-white hover:bg-blue-700"
          >
            Create
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

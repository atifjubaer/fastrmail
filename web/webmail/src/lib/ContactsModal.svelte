<script lang="ts">
  import { onMount } from "svelte";
  import {
    X,
    Search,
    Plus,
    User,
    Mail,
    Trash2,
    Send,
    Phone,
    FileText,
  } from "@lucide/svelte";
  import {
    getContacts,
    addContact,
    deleteContact,
    type Contact,
  } from "./contacts";

  let {
    isOpen = $bindable(false),
    onComposeTo,
  }: {
    isOpen: boolean;
    onComposeTo: (email: string) => void;
  } = $props();

  let searchQuery = $state("");
  let contactsList = $state<Contact[]>([]);
  let isAdding = $state(false);

  // New Contact form
  let newName = $state("");
  let newEmail = $state("");
  let newPhone = $state("");
  let newNotes = $state("");

  function refresh() {
    contactsList = getContacts();
  }

  function handleSaveContact(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim() || !newEmail.trim()) return;

    addContact({
      name: newName.trim(),
      email: newEmail.trim(),
      phone: newPhone.trim() || undefined,
      notes: newNotes.trim() || undefined,
    });

    newName = "";
    newEmail = "";
    newPhone = "";
    newNotes = "";
    isAdding = false;
    refresh();
  }

  function handleDelete(id: string) {
    deleteContact(id);
    refresh();
  }

  let filteredContacts = $derived(
    contactsList.filter((c) => {
      const q = searchQuery.toLowerCase().trim();
      if (!q) return true;
      return (
        c.name.toLowerCase().includes(q) ||
        c.email.toLowerCase().includes(q) ||
        (c.notes && c.notes.toLowerCase().includes(q))
      );
    })
  );

  onMount(() => {
    refresh();
  });
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-xs p-4">
    <div class="flex h-[560px] w-full max-w-2xl flex-col rounded-xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950 overflow-hidden">
      <!-- Modal Header -->
      <div class="flex h-12 items-center justify-between border-b border-zinc-200 px-5 dark:border-zinc-800 bg-zinc-50/70 dark:bg-zinc-900/40">
        <div class="flex items-center gap-2">
          <User class="size-4 text-zinc-500" />
          <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">Address Book & Contacts</span>
          <span class="rounded bg-zinc-200/70 px-1.5 py-0.5 text-[10px] font-mono text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400">
            {contactsList.length}
          </span>
        </div>
        <button
          type="button"
          onclick={() => (isOpen = false)}
          class="rounded p-1 text-zinc-400 hover:bg-zinc-200/50 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-white"
        >
          <X class="size-4" />
        </button>
      </div>

      <!-- Action & Search Toolbar -->
      <div class="flex items-center justify-between gap-3 border-b border-zinc-200 p-3 dark:border-zinc-800 bg-white dark:bg-zinc-950">
        <div class="relative flex-1">
          <Search class="absolute left-2.5 top-2 size-3.5 text-zinc-400" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder="Search contacts by name or email..."
            class="h-7.5 w-full rounded border border-zinc-200 bg-zinc-50 pl-8 pr-3 text-xs outline-none focus:border-zinc-400 focus:bg-white dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
          />
        </div>

        <button
          type="button"
          onclick={() => (isAdding = !isAdding)}
          class="flex h-7.5 items-center gap-1.5 rounded bg-zinc-900 px-3 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-200"
        >
          <Plus class="size-3" />
          <span>{isAdding ? "Cancel" : "Add Contact"}</span>
        </button>
      </div>

      <!-- Optional Add Contact Form Panel -->
      {#if isAdding}
        <form onsubmit={handleSaveContact} class="border-b border-zinc-200 bg-zinc-50/80 p-4 dark:border-zinc-800 dark:bg-zinc-900/60">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label for="contact-name" class="block text-[11px] font-medium text-zinc-500">Full Name</label>
              <input
                id="contact-name"
                type="text"
                bind:value={newName}
                placeholder="Jane Doe"
                required
                class="mt-1 h-7.5 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-700 dark:bg-zinc-800 dark:text-white"
              />
            </div>
            <div>
              <label for="contact-email" class="block text-[11px] font-medium text-zinc-500">Email Address</label>
              <input
                id="contact-email"
                type="email"
                bind:value={newEmail}
                placeholder="jane@example.com"
                required
                class="mt-1 h-7.5 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-700 dark:bg-zinc-800 dark:text-white"
              />
            </div>
          </div>
          <div class="mt-2 grid grid-cols-2 gap-3">
            <div>
              <label for="contact-phone" class="block text-[11px] font-medium text-zinc-500">Phone (Optional)</label>
              <input
                id="contact-phone"
                type="text"
                bind:value={newPhone}
                placeholder="+1 555-0199"
                class="mt-1 h-7.5 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-700 dark:bg-zinc-800 dark:text-white"
              />
            </div>
            <div>
              <label for="contact-notes" class="block text-[11px] font-medium text-zinc-500">Notes / Group</label>
              <input
                id="contact-notes"
                type="text"
                bind:value={newNotes}
                placeholder="Engineering, Ops..."
                class="mt-1 h-7.5 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-700 dark:bg-zinc-800 dark:text-white"
              />
            </div>
          </div>
          <div class="mt-3 flex justify-end gap-2">
            <button
              type="button"
              onclick={() => (isAdding = false)}
              class="h-7 rounded px-3 text-xs text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
            >
              Cancel
            </button>
            <button
              type="submit"
              class="h-7 rounded bg-zinc-900 px-3 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900"
            >
              Save Contact
            </button>
          </div>
        </form>
      {/if}

      <!-- Contact List -->
      <div class="flex-1 overflow-y-auto divide-y divide-zinc-100 dark:divide-zinc-900 p-2">
        {#if filteredContacts.length === 0}
          <div class="flex h-48 flex-col items-center justify-center text-center text-zinc-400">
            <User class="size-8 stroke-[1.2] opacity-40 mb-2" />
            <p class="text-xs">No contacts found</p>
          </div>
        {:else}
          {#each filteredContacts as contact (contact.id)}
            <div class="flex items-center justify-between p-3 rounded-lg hover:bg-zinc-50 dark:hover:bg-zinc-900/60 transition-colors">
              <div class="flex items-center gap-3">
                <div class="flex size-8 items-center justify-center rounded-full bg-zinc-100 text-xs font-semibold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">
                  {contact.name.charAt(0).toUpperCase()}
                </div>
                <div>
                  <div class="text-xs font-semibold text-zinc-900 dark:text-zinc-100">{contact.name}</div>
                  <div class="font-mono text-[11px] text-zinc-500">{contact.email}</div>
                  {#if contact.notes}
                    <div class="text-[10px] text-zinc-400">{contact.notes}</div>
                  {/if}
                </div>
              </div>

              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  title="Compose Email"
                  onclick={() => {
                    isOpen = false;
                    onComposeTo(contact.email);
                  }}
                  class="flex h-7 items-center gap-1 rounded border border-zinc-200 bg-white px-2 text-[11px] font-medium text-zinc-700 shadow-2xs hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300 dark:hover:bg-zinc-800"
                >
                  <Send class="size-3" />
                  <span>Email</span>
                </button>
                <button
                  type="button"
                  title="Delete Contact"
                  onclick={() => handleDelete(contact.id)}
                  class="size-7 flex items-center justify-center rounded text-zinc-400 hover:bg-rose-50 hover:text-rose-600 dark:hover:bg-rose-950/40 dark:hover:text-rose-400"
                >
                  <Trash2 class="size-3.5" />
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

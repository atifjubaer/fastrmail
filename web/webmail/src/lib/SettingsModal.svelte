<script lang="ts">
  import { onMount } from "svelte";
  import {
    X,
    Settings,
    User,
    Folder,
    Keyboard,
    Shield,
    Check,
    Plus,
    Trash2,
    Sliders,
  } from "@lucide/svelte";
  import {
    getSettings,
    saveSettings,
    type UserSettings,
  } from "./settings";

  let {
    isOpen = $bindable(false),
    onSettingsChanged,
    onEmptyFolder,
  }: {
    isOpen: boolean;
    onSettingsChanged: (settings: UserSettings) => void;
    onEmptyFolder: (folder: "Trash" | "Spam") => void;
  } = $props();

  let activeTab = $state<"general" | "identity" | "folders" | "shortcuts">("general");
  let settings = $state<UserSettings>(getSettings());
  let newFolderName = $state("");

  function handleSave() {
    saveSettings(settings);
    onSettingsChanged(settings);
    isOpen = false;
  }

  function handleAddFolder(e: SubmitEvent) {
    e.preventDefault();
    if (!newFolderName.trim()) return;
    const name = newFolderName.trim();
    if (!settings.customFolders.includes(name)) {
      settings.customFolders = [...settings.customFolders, name];
      saveSettings(settings);
      onSettingsChanged(settings);
    }
    newFolderName = "";
  }

  function handleRemoveFolder(name: string) {
    settings.customFolders = settings.customFolders.filter((f) => f !== name);
    saveSettings(settings);
    onSettingsChanged(settings);
  }

  onMount(() => {
    settings = getSettings();
  });
</script>

{#if isOpen}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-xs p-4">
    <div class="flex h-[560px] w-full max-w-2xl flex-col rounded-xl border border-zinc-200 bg-white shadow-2xl dark:border-zinc-800 dark:bg-zinc-950 overflow-hidden">
      <!-- Modal Header -->
      <div class="flex h-12 items-center justify-between border-b border-zinc-200 px-5 dark:border-zinc-800 bg-zinc-50/70 dark:bg-zinc-900/40">
        <div class="flex items-center gap-2">
          <Settings class="size-4 text-zinc-500" />
          <span class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">Webmail Preferences & Settings</span>
        </div>
        <button
          type="button"
          onclick={() => (isOpen = false)}
          class="rounded p-1 text-zinc-400 hover:bg-zinc-200/50 hover:text-zinc-700 dark:hover:bg-zinc-800 dark:hover:text-white"
        >
          <X class="size-4" />
        </button>
      </div>

      <div class="flex flex-1 overflow-hidden">
        <!-- Settings Sidebar Tabs -->
        <div class="w-48 border-r border-zinc-200 bg-zinc-50/50 p-2.5 dark:border-zinc-800 dark:bg-zinc-900/30 flex flex-col gap-1">
          <button
            type="button"
            onclick={() => (activeTab = "general")}
            class="flex h-8 items-center gap-2 rounded px-2.5 text-xs font-medium transition-colors {activeTab === 'general'
              ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
              : 'text-zinc-600 hover:bg-zinc-200/40 dark:text-zinc-400 dark:hover:bg-zinc-800/40'}"
          >
            <Sliders class="size-3.5" />
            <span>General</span>
          </button>

          <button
            type="button"
            onclick={() => (activeTab = "identity")}
            class="flex h-8 items-center gap-2 rounded px-2.5 text-xs font-medium transition-colors {activeTab === 'identity'
              ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
              : 'text-zinc-600 hover:bg-zinc-200/40 dark:text-zinc-400 dark:hover:bg-zinc-800/40'}"
          >
            <User class="size-3.5" />
            <span>Identities & Signatures</span>
          </button>

          <button
            type="button"
            onclick={() => (activeTab = "folders")}
            class="flex h-8 items-center gap-2 rounded px-2.5 text-xs font-medium transition-colors {activeTab === 'folders'
              ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
              : 'text-zinc-600 hover:bg-zinc-200/40 dark:text-zinc-400 dark:hover:bg-zinc-800/40'}"
          >
            <Folder class="size-3.5" />
            <span>Folders Management</span>
          </button>

          <button
            type="button"
            onclick={() => (activeTab = "shortcuts")}
            class="flex h-8 items-center gap-2 rounded px-2.5 text-xs font-medium transition-colors {activeTab === 'shortcuts'
              ? 'bg-zinc-200/80 font-semibold text-zinc-900 dark:bg-zinc-800 dark:text-white'
              : 'text-zinc-600 hover:bg-zinc-200/40 dark:text-zinc-400 dark:hover:bg-zinc-800/40'}"
          >
            <Keyboard class="size-3.5" />
            <span>Shortcuts</span>
          </button>
        </div>

        <!-- Settings Tab Content Area -->
        <div class="flex-1 overflow-y-auto p-6 bg-white dark:bg-zinc-950">
          <!-- ── GENERAL TAB ────────────────────────────────────────────── -->
          {#if activeTab === "general"}
            <div class="flex flex-col gap-5 max-w-md">
              <div>
                <label for="settings-refresh-interval" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">
                  Auto-Check New Mail
                </label>
                <p class="text-[11px] text-zinc-500 mb-2">Interval to check IMAP mailbox updates in background</p>
                <select
                  id="settings-refresh-interval"
                  bind:value={settings.refreshIntervalMinutes}
                  class="h-8 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                >
                  <option value={1}>Every 1 minute</option>
                  <option value={5}>Every 5 minutes</option>
                  <option value={10}>Every 10 minutes</option>
                  <option value={0}>Manual refresh only</option>
                </select>
              </div>

              <div>
                <label for="settings-density" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">
                  Interface Density
                </label>
                <p class="text-[11px] text-zinc-500 mb-2">Display density for high-resolution desktop monitors</p>
                <select
                  id="settings-density"
                  bind:value={settings.density}
                  class="h-8 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                >
                  <option value="compact">Compact (SnappyMail / Roundcube style)</option>
                  <option value="normal">Normal (Standard)</option>
                </select>
              </div>

              <div class="pt-2 border-t border-zinc-100 dark:border-zinc-900">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input
                    type="checkbox"
                    bind:checked={settings.blockExternalImages}
                    class="rounded text-zinc-900"
                  />
                  <div>
                    <span class="text-xs font-medium text-zinc-800 dark:text-zinc-200">Block External Images</span>
                    <p class="text-[11px] text-zinc-500">GDPR Privacy protection against email tracking pixels</p>
                  </div>
                </label>
              </div>
            </div>

          <!-- ── IDENTITY & SIGNATURE TAB ─────────────────────────────────── -->
          {:else if activeTab === "identity"}
            <div class="flex flex-col gap-4 max-w-md">
              <div>
                <label for="identity-display-name" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">Display Name</label>
                <input
                  id="identity-display-name"
                  type="text"
                  bind:value={settings.displayName}
                  placeholder="Your Name"
                  class="mt-1 h-8 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                />
              </div>

              <div>
                <label for="identity-email" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">Sender Email</label>
                <input
                  id="identity-email"
                  type="email"
                  bind:value={settings.email}
                  disabled
                  class="mt-1 h-8 w-full rounded border border-zinc-200 bg-zinc-100 px-2.5 text-xs outline-none opacity-70 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-400"
                />
              </div>

              <div>
                <label for="identity-reply-to" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">Reply-To Address</label>
                <input
                  id="identity-reply-to"
                  type="email"
                  bind:value={settings.replyTo}
                  placeholder="reply@domain.com"
                  class="mt-1 h-8 w-full rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                />
              </div>

              <div>
                <label for="identity-signature" class="block text-xs font-semibold text-zinc-800 dark:text-zinc-200">Email Signature (HTML)</label>
                <p class="text-[11px] text-zinc-500 mb-1">Automatically appended to all outbound emails</p>
                <textarea
                  id="identity-signature"
                  bind:value={settings.signature}
                  rows="4"
                  class="w-full resize-none rounded border border-zinc-200 bg-white p-2.5 font-mono text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                ></textarea>
              </div>
            </div>

          <!-- ── FOLDERS TAB ─────────────────────────────────────────────── -->
          {:else if activeTab === "folders"}
            <div class="flex flex-col gap-4">
              <div>
                <h4 class="text-xs font-semibold text-zinc-800 dark:text-zinc-200">Create Custom IMAP Folder</h4>
                <form onsubmit={handleAddFolder} class="mt-2 flex gap-2 max-w-md">
                  <input
                    type="text"
                    bind:value={newFolderName}
                    placeholder="Folder name (e.g. Clients, Taxes)"
                    class="h-8 flex-1 rounded border border-zinc-200 bg-white px-2.5 text-xs outline-none dark:border-zinc-800 dark:bg-zinc-900 dark:text-white"
                  />
                  <button
                    type="submit"
                    class="flex h-8 items-center gap-1.5 rounded bg-zinc-900 px-3 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900"
                  >
                    <Plus class="size-3" />
                    <span>Create</span>
                  </button>
                </form>
              </div>

              <div class="mt-2">
                <h4 class="text-xs font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Custom Folders</h4>
                <div class="max-w-md divide-y divide-zinc-100 border border-zinc-200 rounded-lg dark:divide-zinc-900 dark:border-zinc-800 overflow-hidden">
                  {#each settings.customFolders as folder}
                    <div class="flex items-center justify-between p-2.5 text-xs text-zinc-700 dark:text-zinc-300">
                      <div class="flex items-center gap-2">
                        <Folder class="size-3.5 text-zinc-400" />
                        <span>{folder}</span>
                      </div>
                      <button
                        type="button"
                        onclick={() => handleRemoveFolder(folder)}
                        class="text-zinc-400 hover:text-rose-500"
                        title="Delete Folder"
                      >
                        <Trash2 class="size-3.5" />
                      </button>
                    </div>
                  {/each}
                </div>
              </div>

              <div class="mt-4 pt-4 border-t border-zinc-100 dark:border-zinc-900 max-w-md">
                <h4 class="text-xs font-semibold text-zinc-800 dark:text-zinc-200 mb-2">Purge Actions</h4>
                <div class="flex items-center gap-3">
                  <button
                    type="button"
                    onclick={() => onEmptyFolder("Trash")}
                    class="flex h-7.5 items-center gap-1.5 rounded border border-zinc-200 bg-white px-3 text-xs text-zinc-700 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300 dark:hover:bg-zinc-800"
                  >
                    <Trash2 class="size-3 text-rose-500" />
                    <span>Empty Trash</span>
                  </button>
                  <button
                    type="button"
                    onclick={() => onEmptyFolder("Spam")}
                    class="flex h-7.5 items-center gap-1.5 rounded border border-zinc-200 bg-white px-3 text-xs text-zinc-700 hover:bg-zinc-50 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300 dark:hover:bg-zinc-800"
                  >
                    <Shield class="size-3 text-amber-500" />
                    <span>Empty Spam</span>
                  </button>
                </div>
              </div>
            </div>

          <!-- ── KEYBOARD SHORTCUTS TAB ──────────────────────────────────── -->
          {:else if activeTab === "shortcuts"}
            <div class="max-w-md">
              <h4 class="text-xs font-semibold text-zinc-800 dark:text-zinc-200 mb-3">SnappyMail Keyboard Shortcuts</h4>
              <div class="grid grid-cols-2 gap-2 text-xs">
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Compose email</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">C</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Reply</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">R</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Reply All</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">A</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Forward</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">F</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Toggle Star</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">S</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Toggle Unread</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">U</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Focus Search</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">/</kbd>
                </div>
                <div class="flex items-center justify-between p-2 rounded bg-zinc-50 dark:bg-zinc-900 border border-zinc-200/60 dark:border-zinc-800/60">
                  <span class="text-zinc-600 dark:text-zinc-400">Send Email</span>
                  <kbd class="rounded bg-zinc-200 px-1.5 py-0.5 font-mono text-[10px] font-bold text-zinc-700 dark:bg-zinc-800 dark:text-zinc-300">Ctrl+Enter</kbd>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="flex h-12 items-center justify-end gap-2 border-t border-zinc-200 px-5 dark:border-zinc-800 bg-zinc-50/70 dark:bg-zinc-900/40">
        <button
          type="button"
          onclick={() => (isOpen = false)}
          class="h-8 rounded px-3 text-xs text-zinc-500 hover:text-zinc-800 dark:hover:text-zinc-200"
        >
          Cancel
        </button>
        <button
          type="button"
          onclick={handleSave}
          class="flex h-8 items-center gap-1.5 rounded bg-zinc-900 px-4 text-xs font-medium text-white hover:bg-zinc-800 dark:bg-zinc-100 dark:text-zinc-900"
        >
          <Check class="size-3" />
          <span>Save Changes</span>
        </button>
      </div>
    </div>
  </div>
{/if}

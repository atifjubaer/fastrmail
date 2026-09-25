<script lang="ts">
  import { onMount } from "svelte";
  import {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    List,
    ListOrdered,
    Quote,
    Code,
    Link as LinkIcon,
    Eraser,
    Undo2,
    Redo2,
    Paperclip,
    X,
    FileText,
    FileCode,
  } from "@lucide/svelte";

  export interface AttachmentFile {
    id: string;
    name: string;
    size: number;
    type: string;
  }

  let {
    html = $bindable("<p></p>"),
    plainText = $bindable(""),
    attachments = $bindable<AttachmentFile[]>([]),
    placeholder = "Write your message here...",
  }: {
    html: string;
    plainText: string;
    attachments: AttachmentFile[];
    placeholder?: string;
  } = $props();

  let editorElement = $state<HTMLDivElement | null>(null);
  let fileInput = $state<HTMLInputElement | null>(null);
  let isSourceMode = $state(false);

  function format(command: string, value: string | undefined = undefined) {
    if (isSourceMode) return;
    document.execCommand(command, false, value);
    syncContent();
    if (editorElement) editorElement.focus();
  }

  function insertLink() {
    if (isSourceMode) return;
    const url = prompt("Enter link URL (e.g. https://example.com):");
    if (url) {
      format("createLink", url);
    }
  }

  function syncContent() {
    if (!editorElement) return;
    html = editorElement.innerHTML;
    plainText = editorElement.innerText || "";
  }

  function handleInput() {
    syncContent();
  }

  function handlePaste(e: ClipboardEvent) {
    // Normal paste behavior preserved
    setTimeout(syncContent, 10);
  }

  function toggleSourceMode() {
    if (isSourceMode) {
      // Switching from source to visual
      if (editorElement) editorElement.innerHTML = html;
      isSourceMode = false;
    } else {
      // Switching from visual to source
      syncContent();
      isSourceMode = true;
    }
  }

  function handleFileSelect(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const files = Array.from(input.files);
    const newAttachments: AttachmentFile[] = files.map((f) => ({
      id: `att-${Date.now()}-${Math.random().toString(36).substring(2, 7)}`,
      name: f.name,
      size: f.size,
      type: f.type || "application/octet-stream",
    }));

    attachments = [...attachments, ...newAttachments];
    input.value = "";
  }

  function removeAttachment(id: string) {
    attachments = attachments.filter((a) => a.id !== id);
  }

  function formatFileSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  onMount(() => {
    if (editorElement) {
      if (html && html !== "<p></p>") {
        editorElement.innerHTML = html;
      }
    }
  });
</script>

<div class="flex flex-1 flex-col overflow-hidden bg-white dark:bg-zinc-950">
  <!-- Roundcube / SnappyMail Rich Formatting Toolbar -->
  <div class="flex flex-wrap items-center gap-0.5 border-b border-zinc-200 bg-zinc-50/80 px-3 py-1.5 dark:border-zinc-800 dark:bg-zinc-900/60">
    <!-- Undo / Redo -->
    <div class="flex items-center">
      <button
        type="button"
        onclick={() => format("undo")}
        title="Undo (Ctrl+Z)"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Undo2 class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("redo")}
        title="Redo (Ctrl+Y)"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Redo2 class="size-3.5" />
      </button>
    </div>

    <div class="mx-1 h-4 w-px bg-zinc-200 dark:bg-zinc-800"></div>

    <!-- Inline Formatting -->
    <div class="flex items-center">
      <button
        type="button"
        onclick={() => format("bold")}
        title="Bold (Ctrl+B)"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Bold class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("italic")}
        title="Italic (Ctrl+I)"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Italic class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("underline")}
        title="Underline (Ctrl+U)"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Underline class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("strikeThrough")}
        title="Strikethrough"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Strikethrough class="size-3.5" />
      </button>
    </div>

    <div class="mx-1 h-4 w-px bg-zinc-200 dark:bg-zinc-800"></div>

    <!-- Paragraph & Heading Formatting -->
    <div class="flex items-center">
      <button
        type="button"
        onclick={() => format("formatBlock", "<h2>")}
        title="Heading"
        class="inline-flex h-7 px-1.5 items-center justify-center rounded text-[11px] font-bold text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        H2
      </button>
      <button
        type="button"
        onclick={() => format("formatBlock", "<h3>")}
        title="Subheading"
        class="inline-flex h-7 px-1.5 items-center justify-center rounded text-[11px] font-semibold text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        H3
      </button>
      <button
        type="button"
        onclick={() => format("formatBlock", "<p>")}
        title="Paragraph"
        class="inline-flex h-7 px-1.5 items-center justify-center rounded text-[11px] font-medium text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        P
      </button>
    </div>

    <div class="mx-1 h-4 w-px bg-zinc-200 dark:bg-zinc-800"></div>

    <!-- Lists & Quotes -->
    <div class="flex items-center">
      <button
        type="button"
        onclick={() => format("insertUnorderedList")}
        title="Bullet List"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <List class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("insertOrderedList")}
        title="Numbered List"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <ListOrdered class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("formatBlock", "<blockquote>")}
        title="Blockquote"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Quote class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("formatBlock", "<pre>")}
        title="Code Block"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Code class="size-3.5" />
      </button>
    </div>

    <div class="mx-1 h-4 w-px bg-zinc-200 dark:bg-zinc-800"></div>

    <!-- Links, Clear & Attachments -->
    <div class="flex items-center">
      <button
        type="button"
        onclick={insertLink}
        title="Insert Link"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <LinkIcon class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => format("removeFormat")}
        title="Clear Formatting"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Eraser class="size-3.5" />
      </button>
      <button
        type="button"
        onclick={() => fileInput?.click()}
        title="Attach File"
        class="inline-flex size-7 items-center justify-center rounded text-zinc-600 hover:bg-zinc-200/70 hover:text-zinc-900 dark:text-zinc-400 dark:hover:bg-zinc-800 dark:hover:text-zinc-100"
      >
        <Paperclip class="size-3.5" />
      </button>
    </div>

    <!-- Hidden file input -->
    <input
      type="file"
      multiple
      bind:this={fileInput}
      onchange={handleFileSelect}
      class="hidden"
    />

    <!-- Right-aligned HTML / Plain Mode Switch -->
    <div class="ml-auto flex items-center">
      <button
        type="button"
        onclick={toggleSourceMode}
        title={isSourceMode ? "Switch to Visual Editor" : "View HTML Source"}
        class="inline-flex h-7 items-center gap-1 rounded px-2 text-[11px] font-mono {isSourceMode ? 'bg-zinc-900 text-white dark:bg-zinc-100 dark:text-zinc-900' : 'text-zinc-500 hover:bg-zinc-200/60 dark:text-zinc-400 dark:hover:bg-zinc-800'}"
      >
        <FileCode class="size-3" />
        <span>{isSourceMode ? "Visual" : "HTML"}</span>
      </button>
    </div>
  </div>

  <!-- Attachments preview strip if any attached -->
  {#if attachments.length > 0}
    <div class="flex flex-wrap items-center gap-1.5 border-b border-zinc-200 bg-zinc-100/50 p-2 dark:border-zinc-800 dark:bg-zinc-900/40">
      {#each attachments as att (att.id)}
        <div class="inline-flex items-center gap-1.5 rounded border border-zinc-200 bg-white px-2 py-1 text-[11px] text-zinc-700 shadow-2xs dark:border-zinc-700 dark:bg-zinc-800 dark:text-zinc-200">
          <FileText class="size-3 text-zinc-400" />
          <span class="max-w-[160px] truncate font-medium">{att.name}</span>
          <span class="text-[10px] text-zinc-400">({formatFileSize(att.size)})</span>
          <button
            type="button"
            onclick={() => removeAttachment(att.id)}
            class="ml-0.5 rounded p-0.5 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 dark:hover:bg-zinc-700 dark:hover:text-zinc-100"
          >
            <X class="size-2.5" />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Editor Body Area -->
  <div class="relative flex-1 overflow-auto p-4">
    {#if isSourceMode}
      <textarea
        bind:value={html}
        oninput={() => {
          if (editorElement) editorElement.innerHTML = html;
          plainText = editorElement?.innerText || "";
        }}
        class="size-full resize-none font-mono text-xs leading-relaxed text-zinc-800 outline-none dark:bg-zinc-950 dark:text-zinc-200"
        placeholder="Enter raw HTML here..."
      ></textarea>
    {:else}
      <!-- Native Contenteditable WYSIWYG surface -->
      <div
        bind:this={editorElement}
        contenteditable="true"
        oninput={handleInput}
        onpaste={handlePaste}
        role="textbox"
        tabindex="0"
        aria-multiline="true"
        data-placeholder={placeholder}
        class="prose prose-sm dark:prose-invert min-h-full max-w-none text-xs leading-relaxed text-zinc-900 outline-none dark:text-zinc-100 [&:empty]:before:pointer-events-none [&:empty]:before:text-zinc-400 [&:empty]:before:content-[attr(data-placeholder)] [&>p]:mb-2 [&>h2]:text-base [&>h2]:font-bold [&>h2]:mb-2 [&>h3]:text-sm [&>h3]:font-semibold [&>h3]:mb-1 [&>ul]:list-disc [&>ul]:ml-4 [&>ol]:list-decimal [&>ol]:ml-4 [&>blockquote]:border-l-2 [&>blockquote]:border-zinc-300 [&>blockquote]:pl-3 [&>blockquote]:italic [&>pre]:rounded [&>pre]:bg-zinc-100 [&>pre]:p-2 [&>pre]:font-mono dark:[&>pre]:bg-zinc-900"
      ></div>
    {/if}
  </div>
</div>

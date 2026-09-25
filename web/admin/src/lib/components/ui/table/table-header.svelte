<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "../../../utils";

  let {
    class: className = "",
    children,
    ...restProps
  }: {
    class?: string;
    children?: Snippet;
    [key: string]: any;
  } = $props();
</script>

<thead class={cn("[&_tr]:border-b border-zinc-800/80 bg-zinc-900/40", className)} {...restProps}>
  {@render children?.()}
</thead>

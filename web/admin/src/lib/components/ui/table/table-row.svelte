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

<tr
  class={cn(
    "border-b border-zinc-800/60 transition-colors hover:bg-zinc-800/30 data-[state=selected]:bg-zinc-800",
    className
  )}
  {...restProps}
>
  {@render children?.()}
</tr>

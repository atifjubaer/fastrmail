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

<th
  class={cn(
    "h-9 px-4 text-left align-middle font-mono text-[11px] font-medium text-zinc-400 [&:has([role=checkbox])]:pr-0",
    className
  )}
  {...restProps}
>
  {@render children?.()}
</th>

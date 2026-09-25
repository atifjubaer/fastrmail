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

<h3
  class={cn("font-medium tracking-tight text-foreground text-xs uppercase text-zinc-400 font-mono", className)}
  {...restProps}
>
  {@render children?.()}
</h3>

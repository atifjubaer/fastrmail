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

<div
  class={cn(
    "rounded-xl border border-border/60 bg-card/60 text-card-foreground shadow-2xs backdrop-blur-xs transition-colors",
    className
  )}
  {...restProps}
>
  {@render children?.()}
</div>

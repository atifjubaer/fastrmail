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

<div class={cn("flex flex-col gap-1.5 p-5 pb-3", className)} {...restProps}>
  {@render children?.()}
</div>

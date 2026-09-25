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

<div class="relative w-full overflow-auto">
  <table class={cn("w-full caption-bottom text-xs", className)} {...restProps}>
    {@render children?.()}
  </table>
</div>

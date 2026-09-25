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

<td
  class={cn("p-4 align-middle [&:has([role=checkbox])]:pr-0 text-xs text-zinc-300", className)}
  {...restProps}
>
  {@render children?.()}
</td>

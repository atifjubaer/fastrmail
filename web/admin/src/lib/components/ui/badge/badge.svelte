<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "../../../utils";

  let {
    variant = "default",
    class: className = "",
    children,
    ...restProps
  }: {
    variant?: "default" | "secondary" | "destructive" | "outline" | "success";
    class?: string;
    children?: Snippet;
    [key: string]: any;
  } = $props();

  const variantStyles = {
    default: "border-transparent bg-zinc-800 text-zinc-100 hover:bg-zinc-700",
    secondary: "border-transparent bg-zinc-800/60 text-zinc-300 hover:bg-zinc-800",
    destructive: "border-transparent bg-rose-500/15 text-rose-400 border border-rose-500/20",
    outline: "text-zinc-300 border border-zinc-700/60 bg-transparent",
    success: "border-emerald-500/20 bg-emerald-500/10 text-emerald-400 border",
  };
</script>

<div
  class={cn(
    "inline-flex items-center rounded-md px-2 py-0.5 text-[11px] font-mono font-medium transition-colors focus:outline-hidden",
    variantStyles[variant] || variantStyles.default,
    className
  )}
  {...restProps}
>
  {@render children?.()}
</div>

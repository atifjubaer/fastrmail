<script lang="ts">
  import type { Snippet } from "svelte";
  import { cn } from "../../../utils";

  let {
    variant = "default",
    size = "default",
    class: className = "",
    children,
    type = "button",
    disabled = false,
    ...restProps
  }: {
    variant?: "default" | "secondary" | "destructive" | "outline" | "ghost" | "link";
    size?: "default" | "sm" | "lg" | "icon";
    class?: string;
    children?: Snippet;
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    [key: string]: any;
  } = $props();

  const variantStyles = {
    default: "bg-zinc-100 text-zinc-900 hover:bg-zinc-200 shadow-2xs font-semibold",
    secondary: "bg-zinc-800 text-zinc-100 hover:bg-zinc-700/80 shadow-2xs",
    destructive: "bg-rose-600 text-white hover:bg-rose-500 shadow-2xs",
    outline: "border border-zinc-800 bg-transparent text-zinc-200 hover:bg-zinc-800/80 hover:text-white",
    ghost: "text-zinc-400 hover:bg-zinc-800/60 hover:text-zinc-100",
    link: "text-zinc-300 underline-offset-4 hover:underline",
  };

  const sizeStyles = {
    default: "h-8 px-3 text-xs",
    sm: "h-7 px-2.5 text-[11px]",
    lg: "h-9 px-4 text-xs",
    icon: "size-7",
  };
</script>

<button
  {type}
  {disabled}
  class={cn(
    "inline-flex items-center justify-center gap-1.5 rounded-md font-medium transition-all active:scale-[0.99] disabled:pointer-events-none disabled:opacity-50 cursor-pointer",
    variantStyles[variant] || variantStyles.default,
    sizeStyles[size] || sizeStyles.default,
    className
  )}
  {...restProps}
>
  {@render children?.()}
</button>

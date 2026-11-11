<!-- GtkButton with libadwaita style classes: flat, suggested-action, destructive-action, pill, circular. -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLButtonAttributes } from "svelte/elements";
  import Icon from "./Icon.svelte";

  interface Props extends Omit<HTMLButtonAttributes, "class"> {
    variant?: "default" | "flat" | "suggested" | "destructive";
    pill?: boolean;
    circular?: boolean;
    icon?: string;
    label?: string;
    /** Tooltip; also the accessible name for icon-only buttons. */
    tooltip?: string;
    class?: string;
    children?: Snippet;
  }

  let {
    variant = "default",
    pill = false,
    circular = false,
    icon,
    label,
    tooltip,
    class: className = "",
    children,
    ...rest
  }: Props = $props();

  const iconOnly = $derived(!!icon && !label && !children);

  const variants = {
    default: "bg-current/10 hover:bg-current/15 active:bg-current/30",
    flat: "bg-transparent hover:bg-current/7 active:bg-current/16",
    suggested: "bg-accent-bg text-accent-fg hover:brightness-110 active:brightness-90",
    destructive: "bg-destructive-bg text-destructive-fg hover:brightness-110 active:brightness-90",
  };
</script>

<button
  type="button"
  title={tooltip}
  aria-label={iconOnly ? tooltip : undefined}
  class="inline-flex shrink-0 items-center justify-center gap-1.5 font-bold transition-[background-color,filter]
         duration-200 disabled:pointer-events-none disabled:opacity-50
         {variants[variant]}
         {pill ? 'min-h-[34px] rounded-full px-8 py-2.5' : circular ? 'size-[34px] rounded-full' : iconOnly ? 'size-[34px] rounded-button' : 'min-h-[34px] rounded-button px-[17px]'}
         {className}"
  {...rest}
>
  {#if icon}<Icon name={icon} />{/if}
  {#if label}<span>{label}</span>{/if}
  {@render children?.()}
</button>

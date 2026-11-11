<!-- AdwButtonRow: an activatable row with a centred label. -->
<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    label: string;
    icon?: string;
    destructive?: boolean;
    disabled?: boolean;
    onclick: () => void;
  }

  let { label, icon, destructive = false, disabled = false, onclick }: Props = $props();
</script>

<button
  type="button"
  class="activatable flex min-h-[50px] w-full items-center justify-center gap-1.5 px-3 font-bold
         disabled:pointer-events-none disabled:opacity-50"
  class:text-destructive={destructive}
  class:text-accent={!destructive}
  {disabled}
  {onclick}
>
  {#if icon}<Icon name={icon} />{/if}
  {label}
</button>

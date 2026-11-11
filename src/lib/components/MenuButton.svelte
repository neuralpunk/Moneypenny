<!-- GtkMenuButton with a popover menu, anchored under the button. -->
<script lang="ts">
  import Button from "./Button.svelte";

  export interface MenuItem {
    label: string;
    onselect: () => void;
    destructive?: boolean;
    disabled?: boolean;
  }

  interface Props {
    icon?: string;
    tooltip: string;
    /** Groups of items, separated by lines. */
    sections: MenuItem[][];
  }

  let { icon = "open-menu", tooltip, sections }: Props = $props();

  const id = $props.id();
  let button: HTMLElement | undefined = $state();
  let menu: HTMLDivElement | undefined = $state();
  let isOpen = $state(false);

  function place() {
    const anchor = button?.querySelector("button") ?? button;
    if (!anchor || !menu) return;
    const r = anchor.getBoundingClientRect();
    const w = menu.offsetWidth;
    const left = Math.min(Math.max(6, r.right - w), window.innerWidth - w - 6);
    menu.style.left = `${left}px`;
    menu.style.top = `${r.bottom + 4}px`;
  }

  // Up/Down/Home/End move between items, like a GTK menu.
  function onkeydown(e: KeyboardEvent) {
    const items = [...(menu?.querySelectorAll<HTMLButtonElement>(".menu-item:not(:disabled)") ?? [])];
    if (!items.length) return;
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const target =
      e.key === "ArrowDown" ? items[(i + 1) % items.length]
      : e.key === "ArrowUp" ? items[(i - 1 + items.length) % items.length]
      : e.key === "Home" ? items[0]
      : e.key === "End" ? items[items.length - 1]
      : undefined;
    if (target) {
      target.focus();
      e.preventDefault();
    }
  }

  function select(item: MenuItem) {
    menu?.hidePopover();
    item.onselect();
  }

  const visible = $derived(sections.map((s) => s.filter(Boolean)).filter((s) => s.length));
</script>

<span bind:this={button} class="contents">
  <Button
    variant="flat"
    {icon}
    {tooltip}
    aria-haspopup="menu"
    aria-expanded={isOpen}
    popovertarget={id}
    class={isOpen ? "!bg-current/16" : ""}
  />
</span>

<div
  bind:this={menu}
  {id}
  popover="auto"
  role="menu"
  tabindex="-1"
  class="popover-menu"
  {onkeydown}
  ontoggle={(e) => {
    isOpen = (e as ToggleEvent).newState === "open";
    if (isOpen) {
      place();
      menu?.querySelector<HTMLElement>(".menu-item:not(:disabled)")?.focus();
    }
  }}
>
  {#each visible as section, i (i)}
    {#if i > 0}<hr />{/if}
    {#each section as item (item.label)}
      <button
        type="button"
        role="menuitem"
        class="menu-item"
        class:destructive={item.destructive}
        disabled={item.disabled}
        onclick={() => select(item)}
      >
        {item.label}
      </button>
    {/each}
  {/each}
</div>

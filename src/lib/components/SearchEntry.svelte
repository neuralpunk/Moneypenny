<!-- GtkSearchEntry: search icon, text, clear button. -->
<script lang="ts">
  import Icon from "./Icon.svelte";

  interface Props {
    value: string;
    placeholder?: string;
    label?: string;
    class?: string;
    input?: HTMLInputElement;
  }

  let {
    value = $bindable(),
    placeholder = "Search",
    label = "Search",
    class: className = "",
    input = $bindable(),
  }: Props = $props();
</script>

<label
  class="flex h-[34px] min-w-0 items-center gap-1.5 rounded-button bg-current/10 px-2
         focus-within:outline-2 focus-within:-outline-offset-2 focus-within:outline-accent/50 {className}"
>
  <Icon name="system-search" class="dim" />
  <input
    bind:this={input}
    bind:value
    type="search"
    {placeholder}
    aria-label={label}
    spellcheck="false"
    autocomplete="off"
    class="min-w-0 flex-1 bg-transparent font-normal outline-none placeholder:text-current/50
           [&::-webkit-search-cancel-button]:hidden"
    onkeydown={(e) => {
      if (e.key === "Escape" && value) {
        value = "";
        e.stopPropagation();
      }
    }}
  />
  {#if value}
    <button
      type="button"
      class="grid size-5 place-items-center rounded-full opacity-70 hover:opacity-100"
      aria-label="Clear search"
      title="Clear search"
      onclick={() => {
        value = "";
        input?.focus();
      }}
    >
      <Icon name="edit-clear" />
    </button>
  {/if}
</label>

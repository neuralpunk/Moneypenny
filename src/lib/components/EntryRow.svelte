<!-- AdwEntryRow / AdwPasswordEntryRow (with `password`). -->
<script lang="ts">
  import type { Snippet } from "svelte";
  import type { HTMLInputAttributes } from "svelte/elements";
  import Button from "./Button.svelte";

  interface Props extends Omit<HTMLInputAttributes, "value" | "type" | "class"> {
    title: string;
    value: string;
    password?: boolean;
    invalid?: boolean;
    monospace?: boolean;
    /** Extra classes for the row itself. */
    rowClass?: string;
    suffix?: Snippet;
  }

  let {
    title,
    value = $bindable(),
    password = false,
    invalid = false,
    monospace = false,
    rowClass = "",
    suffix,
    ...rest
  }: Props = $props();

  let revealed = $state(false);
  const id = $props.id();
</script>

<div class="entry-row {rowClass}" class:invalid>
  <div class="entry-field">
    <input
      {id}
      bind:value
      type={password && !revealed ? "password" : "text"}
      placeholder=" "
      spellcheck="false"
      autocomplete="off"
      aria-invalid={invalid}
      class:monospace
      {...rest}
    />
    <label for={id} class="entry-title">{title}</label>
  </div>
  {#if password}
    <Button
      variant="flat"
      circular
      icon={revealed ? "view-conceal" : "view-reveal"}
      tooltip={revealed ? "Hide Text" : "Show Text"}
      aria-pressed={revealed}
      onclick={() => (revealed = !revealed)}
    />
  {/if}
  {@render suffix?.()}
</div>

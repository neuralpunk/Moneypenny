<!-- AdwPreferencesGroup: heading, optional description, boxed list. -->
<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    title?: string;
    description?: string;
    children: Snippet;
  }

  let { title, description, children }: Props = $props();
</script>

<section class="flex flex-col gap-3">
  {#if title || description}
    <header class="flex flex-col gap-0.5 px-0.5">
      {#if title}<h2 class="heading">{title}</h2>{/if}
      {#if description}<p class="text-[9pt] dim">{description}</p>{/if}
    </header>
  {/if}
  <div class="boxed-list" role="list">
    {@render children()}
  </div>
</section>

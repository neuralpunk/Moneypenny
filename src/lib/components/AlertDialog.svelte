<!-- AdwAlertDialog: heading, body and a row of responses. Esc picks `cancel`. -->
<script lang="ts" generics="R extends string">
  import Button from "./Button.svelte";

  interface Response {
    id: R;
    label: string;
    appearance?: "default" | "suggested" | "destructive";
  }

  interface Props {
    open: boolean;
    heading: string;
    body?: string;
    responses: Response[];
    /** Response chosen by Esc or closing. */
    cancel: R;
    onresponse: (id: R) => void;
  }

  let { open = $bindable(), heading, body, responses, cancel, onresponse }: Props = $props();

  let el: HTMLDialogElement | undefined = $state();
  let chosen: R | null = null;

  $effect(() => {
    if (!el) return;
    if (open && !el.open) {
      chosen = null;
      el.showModal();
    }
    if (!open && el.open) el.close();
  });

  function respond(id: R) {
    chosen = id;
    el?.close();
  }
</script>

<dialog
  bind:this={el}
  class="adw-dialog"
  style:width="372px"
  role="alertdialog"
  aria-label={heading}
  onclose={() => {
    open = false;
    onresponse(chosen ?? cancel);
  }}
>
  {#if open}
    <div class="flex flex-col gap-6 p-6 text-center">
      <div class="flex flex-col gap-2.5">
        <h2 class="title-2 break-words">{heading}</h2>
        {#if body}<p class="text-balance">{body}</p>{/if}
      </div>
      <div class="flex gap-3">
        {#each responses as r (r.id)}
          <Button
            pill
            class="flex-1 !px-3"
            variant={r.appearance === "destructive" ? "destructive" : r.appearance === "suggested" ? "suggested" : "default"}
            label={r.label}
            autofocus={r.id === cancel}
            onclick={() => respond(r.id)}
          />
        {/each}
      </div>
    </div>
  {/if}
</dialog>

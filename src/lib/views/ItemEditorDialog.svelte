<!--
  Create or edit a secret: name, secret value and attributes. Internal
  `xdg:` attributes (schema, content type) are kept but not shown.
-->
<script lang="ts">
  import { untrack } from "svelte";
  import { actions, pending } from "../actions.svelte";
  import { dialogs, type EditorRequest } from "../dialogs.svelte";
  import { errorMessage } from "../keyring";
  import Button from "../components/Button.svelte";
  import ButtonRow from "../components/ButtonRow.svelte";
  import Dialog from "../components/Dialog.svelte";
  import EntryRow from "../components/EntryRow.svelte";
  import PreferencesGroup from "../components/PreferencesGroup.svelte";
  import Spinner from "../components/Spinner.svelte";

  let { request }: { request: EditorRequest } = $props();

  interface Attr {
    id: number;
    key: string;
    value: string;
  }

  const isInternal = (key: string) => key.startsWith("xdg:");
  let nextId = 0;

  // Snapshot the item when the dialog opens; later refreshes don't clobber edits.
  const original = untrack(() => (request.mode === "edit" ? request.item : null));
  let open = $state(true);
  let label = $state(original?.label ?? "");
  let secret = $state("");
  let attrs = $state<Attr[]>(
    Object.entries(original?.attributes ?? {})
      .filter(([k]) => !isInternal(k))
      .map(([key, value]) => ({ id: nextId++, key, value })),
  );
  const hidden = Object.fromEntries(Object.entries(original?.attributes ?? {}).filter(([k]) => isInternal(k)));
  let error = $state<string | null>(null);

  const keys = $derived(attrs.map((a) => a.key.trim()));
  const duplicate = (i: number) => keys[i] !== "" && keys.indexOf(keys[i]!) !== i;
  const attrsValid = $derived(keys.every((k, i) => k !== "" && !duplicate(i)));
  const valid = $derived(label.trim() !== "" && (original !== null || secret !== "") && attrsValid);
  const busy = $derived(pending.has("save-item"));

  function attributes(): Record<string, string> {
    return { ...hidden, ...Object.fromEntries(attrs.map((a) => [a.key.trim(), a.value])) };
  }

  function sameAttributes(a: Record<string, string>, b: Record<string, string>) {
    const ka = Object.keys(a);
    return ka.length === Object.keys(b).length && ka.every((k) => a[k] === b[k]);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!valid || busy) return;
    error = null;
    let result;
    if (request.mode === "create") {
      result = await actions.createItem(request.collectionId, label, attributes(), secret);
    } else {
      const item = request.item;
      const attrsNow = attributes();
      result = await actions.updateItem(item, {
        label: label.trim() !== item.label ? label : undefined,
        attributes: sameAttributes(attrsNow, item.attributes) ? undefined : attrsNow,
        secret: secret !== "" ? secret : undefined,
      });
    }
    if (result.ok) open = false;
    else if (result.error.kind !== "dismissed") error = errorMessage(result.error);
  }

  function closed() {
    secret = "";
    dialogs.editor = null;
  }
</script>

<Dialog bind:open title={original ? "Edit Secret" : "New Secret"} onclose={closed}>
  {#snippet start()}
    <Button label="Cancel" onclick={() => (open = false)} />
  {/snippet}
  {#snippet end()}
    {#if busy}<Spinner size={16} />{/if}
    <Button
      variant="suggested"
      label={original ? "Save" : "Add"}
      type="submit"
      form="item-editor"
      disabled={!valid || busy}
    />
  {/snippet}

  <form id="item-editor" onsubmit={submit} class="flex flex-col gap-6">
    {#if error}
      <p role="alert" class="rounded-card bg-error/10 px-3 py-2 text-error">{error}</p>
    {/if}

    <PreferencesGroup
      description={original ? "Leave the secret empty to keep the current one." : undefined}
    >
      <EntryRow title="Name" bind:value={label} autofocus maxlength={512} />
      <EntryRow
        title={original ? "New Secret" : "Secret"}
        bind:value={secret}
        password
        monospace
      />
    </PreferencesGroup>

    <PreferencesGroup
      title="Attributes"
      description={original
        ? "Apps use these to find their secrets. Changing them may stop an app from finding this one."
        : "Optional details, like a username or website, that help you and apps find this secret."}
    >
      {#each attrs as attr, i (attr.id)}
        <div class="flex items-stretch">
          <EntryRow
            title="Name"
            bind:value={attr.key}
            rowClass="w-2/5 shrink-0"
            invalid={attr.key.trim() === "" || duplicate(i)}
            aria-label="Attribute name"
          />
          <EntryRow title="Value" bind:value={attr.value} rowClass="min-w-0 flex-1 !pr-1.5" aria-label="Attribute value">
            {#snippet suffix()}
              <Button
                variant="flat"
                circular
                icon="list-remove"
                tooltip="Remove Attribute"
                onclick={() => (attrs = attrs.filter((a) => a.id !== attr.id))}
              />
            {/snippet}
          </EntryRow>
        </div>
      {/each}
      <ButtonRow
        icon="list-add"
        label="Add Attribute"
        onclick={() => (attrs = [...attrs, { id: nextId++, key: "", value: "" }])}
      />
    </PreferencesGroup>
  </form>
</Dialog>

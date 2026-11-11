<!-- GNOME-style Keyboard Shortcuts dialog. -->
<script lang="ts">
  import { dialogs } from "../dialogs.svelte";
  import { SHORTCUT_GROUPS } from "../shortcuts";
  import Button from "../components/Button.svelte";
  import Dialog from "../components/Dialog.svelte";
  import PreferencesGroup from "../components/PreferencesGroup.svelte";
</script>

<Dialog bind:open={dialogs.shortcuts} title="Keyboard Shortcuts" width={460}>
  {#snippet end()}
    <Button variant="flat" circular icon="window-close" tooltip="Close" onclick={() => (dialogs.shortcuts = false)} />
  {/snippet}
  <div class="flex flex-col gap-6">
    {#each SHORTCUT_GROUPS as group (group.title)}
      <PreferencesGroup title={group.title}>
        {#each group.items as s (s.title)}
          <div class="flex min-h-[50px] items-center gap-3 px-3" role="listitem">
            <span class="flex-1">{s.title}</span>
            <span class="flex flex-wrap items-center justify-end gap-1.5">
              {#each s.keys as combo, i (i)}
                {#if i > 0}<span class="text-[9pt] dim">or</span>{/if}
                <span class="flex gap-1">
                  {#each combo as cap (cap)}<kbd class="keycap">{cap}</kbd>{/each}
                </span>
              {/each}
            </span>
          </div>
        {/each}
      </PreferencesGroup>
    {/each}
  </div>
</Dialog>

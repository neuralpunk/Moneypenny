<!-- AdwAboutDialog-style About window. -->
<script lang="ts">
  import appIcon from "../../../src-tauri/icons/moneypenny.svg?url";
  import { dialogs } from "../dialogs.svelte";
  import ActionRow from "../components/ActionRow.svelte";
  import Button from "../components/Button.svelte";
  import Dialog from "../components/Dialog.svelte";
  import PreferencesGroup from "../components/PreferencesGroup.svelte";
</script>

<Dialog bind:open={dialogs.about} title="About Moneypenny" width={400}>
  {#snippet end()}
    <Button variant="flat" circular icon="window-close" tooltip="Close" onclick={() => (dialogs.about = false)} />
  {/snippet}
  <div class="flex flex-col gap-6">
    <div class="flex flex-col items-center gap-3 pt-3 text-center">
      <img src={appIcon} alt="" width="128" height="128" />
      <h1 class="title-1">Moneypenny</h1>
      <span class="rounded-full bg-accent-bg/15 px-3 py-0.5 text-[9pt] font-bold text-accent">{__APP_VERSION__}</span>
      <p class="text-balance">Manage the passwords and secrets in your keyrings.</p>
    </div>
    <PreferencesGroup title="Details">
      <ActionRow property title="Keyring Service" subtitle="oo7 (freedesktop Secret Service)" />
      <ActionRow property title="License" subtitle="GNU General Public License v3.0 or later" />
    </PreferencesGroup>
    <PreferencesGroup title="Credits">
      <ActionRow property title="Built With" subtitle="oo7, Tauri, Svelte and Tailwind CSS" />
      <ActionRow property title="Icons" subtitle="Adwaita icon theme by the GNOME Project (LGPL-3.0 or CC-BY-SA-3.0)" />
    </PreferencesGroup>
  </div>
</Dialog>

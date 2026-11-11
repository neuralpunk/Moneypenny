<!-- Status page for a KeyringError, with a retry action where it helps. -->
<script lang="ts">
  import type { KeyringError } from "../keyring";
  import Button from "../components/Button.svelte";
  import StatusPage from "../components/StatusPage.svelte";

  interface Props {
    error: KeyringError;
    onretry?: () => void;
    compact?: boolean;
  }

  let { error, onretry, compact = false }: Props = $props();

  const page = $derived.by(() => {
    switch (error.kind) {
      case "unavailable":
        return {
          icon: "dialog-error",
          title: "Can't Reach the Keyring Service",
          description: "Moneypenny couldn't connect to oo7. Check that oo7-daemon is running, then try again.",
        };
      case "not-found":
        return {
          icon: "dialog-warning",
          title: "Not Found",
          description: "This keyring or item no longer exists. It may have been deleted by another app.",
        };
      case "locked":
        return { icon: "system-lock-screen", title: "Keyring Locked", description: "Unlock the keyring to continue." };
      case "dismissed":
        return { icon: "dialog-warning", title: "Request Cancelled", description: "The request was dismissed." };
      case "invalid":
      case "backend":
        return { icon: "dialog-warning", title: "Something Went Wrong", description: error.message };
    }
  });
</script>

<StatusPage icon={page.icon} title={page.title} description={page.description} {compact}>
  {#if onretry}
    <Button pill variant="suggested" label="Try Again" onclick={onretry} />
  {/if}
</StatusPage>

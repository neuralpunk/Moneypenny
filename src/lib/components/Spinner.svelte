<!-- AdwSpinner -->
<script lang="ts">
  let { size = 32, label = "Loading" }: { size?: number; label?: string } = $props();
</script>

<svg
  role="progressbar"
  aria-label={label}
  width={size}
  height={size}
  viewBox="0 0 32 32"
  class="animate-spin [animation-duration:1.1s]"
>
  <circle cx="16" cy="16" r="13" fill="none" stroke="currentColor" stroke-opacity=".15" stroke-width="3.5" />
  <path d="M16 3a13 13 0 0 1 13 13" fill="none" stroke="var(--accent-color)" stroke-width="3.5" stroke-linecap="round" />
</svg>

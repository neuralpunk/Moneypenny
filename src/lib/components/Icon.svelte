<!--
  Renders a bundled Adwaita symbolic icon, recoloured with currentColor the way
  GTK recolours symbolic icons.
-->
<script lang="ts" module>
  const icons = import.meta.glob<string>("../icons/*-symbolic.svg", {
    query: "?url",
    import: "default",
    eager: true,
  });

  export function iconUrl(name: string): string | undefined {
    return icons[`../icons/${name}-symbolic.svg`];
  }
</script>

<script lang="ts">
  interface Props {
    /** Icon name without the `-symbolic` suffix, e.g. "window-close". */
    name: string;
    size?: number;
    class?: string;
  }

  let { name, size = 16, class: className = "" }: Props = $props();

  const url = $derived(iconUrl(name));
  // Vite inlines small SVGs as data: URIs containing quotes, so always quote.
  const mask = $derived(url ? `url(${JSON.stringify(url)})` : "none");
  $effect(() => {
    if (import.meta.env.DEV && !url) console.warn(`[Icon] unknown icon "${name}"`);
  });
</script>

<span
  aria-hidden="true"
  class="inline-block shrink-0 bg-current {className}"
  style:width="{size}px"
  style:height="{size}px"
  style:mask-image={mask}
  style:-webkit-mask-image={mask}
  style:mask-size="contain"
  style:-webkit-mask-size="contain"
  style:mask-repeat="no-repeat"
  style:-webkit-mask-repeat="no-repeat"
  style:mask-position="center"
  style:-webkit-mask-position="center"
></span>

<script lang="ts">
  // Werkzeugtaste der Kopfzeilen; `acc` färbt den Balken oben, `on` markiert aktive Modi.
  import type { Component } from 'svelte'
  let { icon: Icon, label, on = false, disabled = false, onclick, title = '', acc = '' }: {
    icon: Component<any>; label: string; on?: boolean; disabled?: boolean; onclick?: () => void; title?: string; acc?: string
  } = $props()
</script>

<button class="tool" class:on {disabled} {onclick} title={title || label} style={acc ? `--acc:${acc}` : ""}>
  <Icon size={20} strokeWidth={1.75} />
  <span>{label}</span>
</button>

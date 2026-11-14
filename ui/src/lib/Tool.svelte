<script lang="ts">
  import type { Component } from 'svelte'
  let { icon: Icon, label, on = false, disabled = false, onclick, title = '' }: {
    icon: Component<any>; label: string; on?: boolean; disabled?: boolean; onclick?: () => void; title?: string
  } = $props()
</script>

<button class="tool" class:on {disabled} {onclick} title={title || label}>
  <Icon size={20} strokeWidth={1.75} />
  <span>{label}</span>
</button>

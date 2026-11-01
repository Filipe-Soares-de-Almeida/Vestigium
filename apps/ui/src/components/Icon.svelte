<script lang="ts" module>
  // Line icons: 16 px for navigation and buttons, 20 px for icon chips.
  const ICONS = {
    today: [16, '<rect x="2" y="3" width="12" height="11" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M2 6.5h12M5 1.5v3M11 1.5v3" stroke="currentColor" stroke-width="1.2"/>'],
    now: [16, '<path d="M1.5 9h3l2-5 3 9 2-4h3" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>'],
    health: [16, '<path d="M8 14s-5.5-3.2-5.5-7.2A3 3 0 018 5a3 3 0 015.5 1.8C13.5 10.8 8 14 8 14z" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    dumps: [16, '<path d="M3 2h7l3 3v9H3z" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    settings: [16, '<circle cx="8" cy="8" r="2.2" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="currentColor" stroke-width="1.2"/>'],
    copy: [16, '<rect x="5" y="2" width="9" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M3 5v7.5A2 2 0 005 14.5h6" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>'],
    back: [16, '<path d="M10 3L5 8l5 5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>'],
    chevron: [12, '<path d="M4.5 2.5L8 6l-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/>'],
    paused: [20, '<circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M8 7v6M12 7v6" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>'],
    stopped: [20, '<circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.8"/><rect x="7" y="7" width="6" height="6" rx="1.2" fill="currentColor"/>'],
    unreachable: [20, '<circle cx="10" cy="10" r="8" fill="none" stroke="currentColor" stroke-width="1.8"/><path d="M10 6v5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/><circle cx="10" cy="14" r="1.1" fill="currentColor"/>'],
    disk: [20, '<rect x="2.5" y="5" width="15" height="10" rx="3" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M6 11.5h5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><circle cx="14" cy="11.5" r="1" fill="currentColor"/>'],
    gpu: [20, '<rect x="2" y="5.5" width="16" height="9" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.6"/><circle cx="8" cy="10" r="2.4" fill="none" stroke="currentColor" stroke-width="1.6"/>'],
    app: [20, '<rect x="2.5" y="3.5" width="15" height="13" rx="3" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M2.5 7.5h15" stroke="currentColor" stroke-width="1.6"/>'],
    chip: [20, '<rect x="5.5" y="5.5" width="9" height="9" rx="2" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M8 2.5v2.5M12 2.5v2.5M8 15v2.5M12 15v2.5M2.5 8h2.5M2.5 12h2.5M15 8h2.5M15 12h2.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>'],
    memory: [20, '<rect x="2.5" y="6.5" width="15" height="7" rx="2" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M6 13.5v2.5M10 13.5v2.5M14 13.5v2.5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>'],
  } as const satisfies Record<string, readonly [number, string]>;

  export type IconName = keyof typeof ICONS;
</script>

<script lang="ts">
  let { name }: { name: IconName } = $props();
  const [size, body] = $derived(ICONS[name]);
</script>

<svg width={size} height={size} viewBox="0 0 {size} {size}" aria-hidden="true" focusable="false">
  {@html body}
</svg>

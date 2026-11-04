<script lang="ts" module>
  // Line icons: 16 px for navigation and buttons, 20 px for icon chips.
  const ICONS = {
    today: [16, '<rect x="2" y="3" width="12" height="11" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M2 6.5h12M5 1.5v3M11 1.5v3" stroke="currentColor" stroke-width="1.2"/>'],
    now: [16, '<path d="M1.5 9h3l2-5 3 9 2-4h3" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round"/>'],
    captures: [16, '<path d="M1.5 8h2.5l1.3-3.5 2 7 1.4-3.5h1.1M11.6 8h2.9" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'],
    health: [16, '<path d="M8 14s-5.5-3.2-5.5-7.2A3 3 0 018 5a3 3 0 015.5 1.8C13.5 10.8 8 14 8 14z" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    dumps: [16, '<path d="M3 2h7l3 3v9H3z" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    settings: [16, '<circle cx="8" cy="8" r="2.2" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="currentColor" stroke-width="1.2"/>'],
    copy: [16, '<rect x="5" y="2" width="9" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M3 5v7.5A2 2 0 005 14.5h6" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>'],
    arrow_left: [16, '<path d="M13 8H3.5M7.5 4L3.5 8l4 4" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
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
    shield: [20, '<path d="M10 2.5l6 2.3v4.5c0 3.8-2.5 6.5-6 8.2-3.5-1.7-6-4.4-6-8.2V4.8z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M7.5 10l1.8 1.8L12.8 8" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
    play: [20, '<rect x="2.5" y="4" width="15" height="11" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M8.5 7.6v4.8l3.8-2.4z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>'],
    pc: [20, '<rect x="2.5" y="3.5" width="15" height="10" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7 16.5h6M10 13.5v3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>'],
    wave: [20, '<path d="M2 10h3l1.6-4.5 2.4 9 1.8-4.5h1.4l1.4 2.5 1.4-5 1.2 2.5H18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
    power: [20, '<path d="M10 3v6.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/><path d="M6 5.6a6 6 0 108 0" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>'],
    clock: [20, '<circle cx="10" cy="10" r="7.5" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M10 6v4.2l2.8 1.6" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>'],
    info: [20, '<circle cx="10" cy="10" r="7.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M10 9v4.5M10 6.6v.1" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/>'],
    tweaks: [16, '<path d="M2.5 4.5h6M11.5 4.5h2M2.5 11.5h2M7.5 11.5h6" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><circle cx="10" cy="4.5" r="1.6" fill="none" stroke="currentColor" stroke-width="1.2"/><circle cx="6" cy="11.5" r="1.6" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    undo: [16, '<path d="M5.5 3.5L2.5 6.5l3 3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/><path d="M2.5 6.5h7a3.5 3.5 0 010 7h-2.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round"/>'],
    lock: [16, '<rect x="3.5" y="7" width="9" height="6.5" rx="1.6" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M5.5 7V5.2a2.5 2.5 0 015 0V7" fill="none" stroke="currentColor" stroke-width="1.2"/>'],
    person: [16, '<circle cx="8" cy="5.5" r="2.4" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M3.2 13.5c.6-2.6 2.5-4 4.8-4s4.2 1.4 4.8 4" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>'],
    restart: [16, '<path d="M12.6 8.2A4.6 4.6 0 1111.2 4.8" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/><path d="M11.8 2.4v2.9H8.9" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'],
    live: [20, '<rect x="2.5" y="4" width="15" height="10" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.6"/><path d="M7 17h6M10 14v3M6 9.5h2l1.2-2.5 1.6 5 1.2-2.5H14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>'],
    trash: [16, '<path d="M2.5 4h11M6.3 4V2.6h3.4V4M4.3 4l.7 9.5h6l.7-9.5" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>'],
    tests: [16, '<rect x="2" y="2" width="12" height="12" rx="3" fill="none" stroke="currentColor" stroke-width="1.2"/><path d="M5 8.2l2 2L11 6" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>'],
    ssd: [20, '<rect x="3" y="5" width="14" height="10" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M6 10h5M13.5 10h.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>'],
    hdd: [20, '<rect x="3" y="4" width="14" height="12" rx="2.5" fill="none" stroke="currentColor" stroke-width="1.5"/><circle cx="10" cy="10" r="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M14 13.5h.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>'],
    scan: [20, '<rect x="3" y="3" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M3 10h14" stroke="currentColor" stroke-width="1.5"/><path d="M7 3v14M13 3v14" stroke="currentColor" stroke-width="1.5" opacity=".55"/>'],
    fsck: [20, '<path d="M5 3h7l3 3v11H5z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M7.5 11l1.8 1.8 3.4-3.6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
    gauge: [20, '<path d="M3.5 14a6.5 6.5 0 1113 0" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M10 14l3.2-4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>'],
    smart: [20, '<path d="M2 10h3.5l2-4.5 3 9 2-4.5H18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
    windows: [20, '<rect x="3" y="3" width="6" height="6" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="11" y="3" width="6" height="6" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="3" y="11" width="6" height="6" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/><rect x="11" y="11" width="6" height="6" rx="1" fill="none" stroke="currentColor" stroke-width="1.5"/>'],
    outside: [20, '<path d="M8 4H5a2 2 0 00-2 2v9a2 2 0 002 2h9a2 2 0 002-2v-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><path d="M11 3h6v6M17 3l-7.5 7.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>'],
    start: [16, '<path d="M5 3.5l7.5 4.5L5 12.5z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>'],
    stop: [16, '<rect x="4" y="4" width="8" height="8" rx="1.6" fill="none" stroke="currentColor" stroke-width="1.3"/>'],
    resume: [16, '<path d="M4 3.5v9M7.5 3.5l6 4.5-6 4.5z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>'],
    write: [16, '<path d="M3 13l.8-3.2 6.6-6.6 2.4 2.4-6.6 6.6z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/>'],
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

export type Channel = {
  enabled: boolean;
  name: string;
  source: string;
  min_duty: number;
  max_duty: number;
  response_ms: number;
  scale: number;
  input_min: number;
  reverse: boolean;
  curve?: number;
  device?: string;
  product_id?: number;
  store_id?: number;
  product_name?: string;
  store_name?: string;
  [key: string]: unknown;
};
export type Config = { version: number; channels: Channel[]; [key: string]: unknown };
export type Board = {
  positions?: number[];
  duties?: number[];
  available?: boolean[];
  calibrating?: number;
  clock_valid?: boolean;
  wifi_connected?: boolean;
  ssid?: string;
  wifi_count?: number;
  ble_count?: number;
  temperature?: number;
  rssi?: number;
  networks?: { ssid: string; rssi: number; open: boolean }[];
  scanning?: boolean;
};
export type Snapshot = {
  connected: boolean;
  device: string;
  firmware: string;
  candidates: string[];
  path: string;
  devices: { id: string; path: string }[];
  config: Config | null;
  metrics: Record<string, number>;
  sources: Source[];
  board: Board;
  paused: boolean;
  error: string | null;
};
export const emptyChannel = (): Channel => ({
  enabled: false,
  name: '',
  source: 'cpu',
  min_duty: 0,
  max_duty: 0,
  response_ms: 500,
  scale: 100,
  input_min: 0,
  reverse: false,
});
export const emptyConfig = (): Config => ({ version: 2, channels: Array.from({ length: 6 }, emptyChannel) });
export const emptySnapshot = (): Snapshot => ({
  connected: false,
  device: '',
  firmware: '',
  candidates: [],
  path: '',
  devices: [],
  config: null,
  metrics: {},
  sources: [],
  board: {},
  paused: false,
  error: null,
});
export type Source = {
  id: string;
  name: string;
  group: string;
  unit: string;
  scale: number;
  description: string;
  detail?: string;
  minimum?: number;
  options?: { id: string; name: string }[];
};
export const sources: Source[] = [
  {
    id: 'cpu',
    name: 'CPU usage',
    group: 'This computer',
    unit: '%',
    scale: 100,
    description: 'How hard your processor is working.',
  },
  {
    id: 'memory',
    name: 'Memory',
    group: 'This computer',
    unit: '%',
    scale: 100,
    description: 'The share of system memory in use.',
  },
  {
    id: 'swap',
    name: 'Swap',
    group: 'This computer',
    unit: '%',
    scale: 100,
    description: 'The share of swap memory in use.',
  },
  {
    id: 'disk',
    name: 'Disk space',
    group: 'This computer',
    unit: '%',
    scale: 100,
    description: 'Space used on your system drive.',
  },
  {
    id: 'network_down',
    name: 'Download',
    group: 'This computer',
    unit: 'MiB/s',
    scale: 10,
    description: 'Incoming traffic across network interfaces.',
  },
  {
    id: 'network_up',
    name: 'Upload',
    group: 'This computer',
    unit: 'MiB/s',
    scale: 10,
    description: 'Outgoing traffic across network interfaces.',
  },
  {
    id: 'battery',
    name: 'Battery',
    group: 'This computer',
    unit: '%',
    scale: 100,
    description: 'Your computer’s remaining charge.',
  },
  {
    id: 'time_day',
    name: '24-hour day',
    group: 'Clock',
    unit: '',
    scale: 24,
    description: 'One slow sweep from midnight to midnight.',
  },
  {
    id: 'time_hours',
    name: 'Hours',
    group: 'Clock',
    unit: '',
    scale: 12,
    description: 'A smooth 12-hour sweep. Pair with minutes and seconds.',
  },
  {
    id: 'time_minutes',
    name: 'Minutes',
    group: 'Clock',
    unit: '',
    scale: 60,
    description: 'One sweep every hour, from 00 to 60.',
  },
  {
    id: 'time_seconds',
    name: 'Seconds',
    group: 'Clock',
    unit: '',
    scale: 60,
    description: 'One sweep every minute, from 00 to 60.',
  },
  {
    id: 'esp_wifi',
    name: 'Wi-Fi networks',
    group: 'ESP32 board',
    unit: 'networks',
    scale: 30,
    description: 'Nearby networks seen by the ESP32. Refreshes every 15 seconds.',
  },
  {
    id: 'esp_ble',
    name: 'Bluetooth devices',
    group: 'ESP32 board',
    unit: 'devices',
    scale: 20,
    description: 'Nearby Bluetooth LE advertisers. Refreshes every 15 seconds.',
    detail: 'Counts advertising addresses, up to 128. Quiet and non-advertising devices are not visible.',
  },
  {
    id: 'esp_temperature',
    name: 'Chip temperature',
    group: 'ESP32 board',
    unit: '°C',
    scale: 100,
    description: 'The ESP32’s internal temperature.',
    detail:
      'Experimental on this ESP32. This is die temperature, not room temperature; accuracy is not guaranteed.',
  },
  {
    id: 'esp_rssi',
    name: 'Wi-Fi strength',
    group: 'ESP32 board',
    unit: '%',
    scale: 100,
    description: 'The strength of the board’s Wi-Fi connection.',
  },
  {
    id: 'constant',
    name: 'Fixed position',
    group: 'ESP32 board',
    unit: '%',
    scale: 50,
    description: 'Hold the needle at a chosen position.',
  },
];
sources.push(
  ...[
    ['wave_sine', 'Sine wave', 'A smooth back-and-forth sweep.'],
    ['wave_triangle', 'Triangle wave', 'A steady sweep in both directions.'],
    ['wave_saw', 'Sawtooth', 'Rise steadily, then return to the start.'],
    ['wave_square', 'Square wave', 'Alternate between the two ends of your gauge.'],
  ].map(([id, name, description]) => ({ id, name, description, group: 'Waveforms', unit: '%', scale: 100 })),
);
export const sourceFor = (id: string, status?: Snapshot): Source =>
  [...(status?.sources ?? []), ...sources].find((s) => s.id === id) ?? {
    id,
    name: id === 'supertracker_coverage' ? 'Basket coverage (removed)' : id,
    group: 'This computer',
    unit: '',
    scale: 100,
    description:
      id === 'supertracker_coverage'
        ? 'Basket coverage has been removed. Choose a food-price or product source.'
        : 'This source needs a compatible provider.',
  };
export function reading(c: Channel, status: Snapshot): number | undefined {
  if (c.source === 'esp_wifi')
    return status.board.wifi_count != null && status.board.wifi_count >= 0
      ? status.board.wifi_count
      : undefined;
  if (c.source === 'esp_ble')
    return status.board.ble_count != null && status.board.ble_count >= 0 ? status.board.ble_count : undefined;
  if (c.source === 'esp_temperature') return status.board.temperature;
  if (c.source === 'esp_rssi')
    return status.board.rssi != null ? Math.min(100, Math.max(0, 2 * (status.board.rssi + 100))) : undefined;
  if (c.source === 'constant') return c.scale;
  return status.metrics[sampleKey(c)];
}

export function sampleKey(c: Channel): string {
  if (c.source === 'supertracker_product') return `product:${c.product_id ?? 0}:${c.store_id ?? 0}`;
  if (
    ['disk', 'network_down', 'network_up', 'gpu', 'cpu_temperature', 'gpu_temperature'].includes(c.source) &&
    c.device
  )
    return `${c.source}:${c.device}`;
  return c.source;
}
export function availableSources(status: Snapshot): Source[] {
  const combined = new Map(sources.map((s) => [s.id, s]));
  for (const source of status.sources) combined.set(source.id, source);
  if (status.metrics.battery == null) combined.delete('battery');
  return [...combined.values()];
}
export function inverseCurve(position: number, strength = 0): number {
  return Math.abs(strength) < 0.001 ? position : Math.log1p(position * Math.expm1(strength)) / strength;
}

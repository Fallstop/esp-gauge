export type Channel = {
  enabled: boolean;
  name: string;
  source: string;
  max_duty: number;
  response_ms: number;
  scale: number;
  reverse: boolean;
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
  path: string;
  devices: { id: string; path: string }[];
  config: Config | null;
  metrics: Record<string, number>;
  board: Board;
  paused: boolean;
  error: string | null;
};
export const emptyChannel = (): Channel => ({
  enabled: false,
  name: '',
  source: 'cpu',
  max_duty: 0,
  response_ms: 500,
  scale: 100,
  reverse: false,
});
export const emptyConfig = (): Config => ({ version: 2, channels: Array.from({ length: 6 }, emptyChannel) });
export const emptySnapshot = (): Snapshot => ({
  connected: false,
  device: '',
  path: '',
  devices: [],
  config: null,
  metrics: {},
  board: {},
  paused: false,
  error: null,
});
export type Source = {
  id: string;
  name: string;
  group: 'Computer' | 'Clock' | 'On board';
  unit: string;
  scale: number;
  description: string;
  detail?: string;
};
export const sources: Source[] = [
  {
    id: 'cpu',
    name: 'CPU usage',
    group: 'Computer',
    unit: '%',
    scale: 100,
    description: 'How hard your processor is working.',
  },
  {
    id: 'memory',
    name: 'Memory',
    group: 'Computer',
    unit: '%',
    scale: 100,
    description: 'The share of system memory in use.',
  },
  {
    id: 'swap',
    name: 'Swap',
    group: 'Computer',
    unit: '%',
    scale: 100,
    description: 'The share of swap memory in use.',
  },
  {
    id: 'disk',
    name: 'Disk space',
    group: 'Computer',
    unit: '%',
    scale: 100,
    description: 'Space used on your system drive.',
  },
  {
    id: 'network_down',
    name: 'Download',
    group: 'Computer',
    unit: 'MiB/s',
    scale: 10,
    description: 'Incoming traffic across network interfaces.',
  },
  {
    id: 'network_up',
    name: 'Upload',
    group: 'Computer',
    unit: 'MiB/s',
    scale: 10,
    description: 'Outgoing traffic across network interfaces.',
  },
  {
    id: 'battery',
    name: 'Battery',
    group: 'Computer',
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
    group: 'On board',
    unit: 'networks',
    scale: 30,
    description: 'Nearby networks seen by the ESP32. Refreshes every 15 seconds.',
  },
  {
    id: 'esp_ble',
    name: 'Bluetooth devices',
    group: 'On board',
    unit: 'devices',
    scale: 20,
    description: 'Nearby Bluetooth LE advertisers. Refreshes every 15 seconds.',
    detail: 'Counts advertising addresses, up to 128. Quiet and non-advertising devices are not visible.',
  },
  {
    id: 'esp_temperature',
    name: 'Chip temperature',
    group: 'On board',
    unit: '°C',
    scale: 100,
    description: 'The ESP32’s internal temperature.',
    detail:
      'Experimental on this ESP32. This is die temperature, not room temperature; accuracy is not guaranteed.',
  },
  {
    id: 'esp_rssi',
    name: 'Wi-Fi strength',
    group: 'On board',
    unit: '%',
    scale: 100,
    description: 'The strength of the board’s Wi-Fi connection.',
  },
  {
    id: 'constant',
    name: 'Fixed position',
    group: 'On board',
    unit: '%',
    scale: 50,
    description: 'Hold the needle at a chosen position.',
  },
];
export const sourceFor = (id: string): Source =>
  sources.find((s) => s.id === id) ?? {
    id,
    name: id,
    group: 'Computer',
    unit: '',
    scale: 100,
    description: 'This source needs a compatible provider.',
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
  return status.metrics[c.source];
}

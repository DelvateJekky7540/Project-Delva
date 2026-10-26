import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const island = document.getElementById('dynamic-island') as HTMLElement;

// --- 1. Logika Buka/Tutup ---
function toggleIsland() {
  const compactView = document.getElementById('compact-view') as HTMLElement;
  const expandedView = document.getElementById('expanded-view') as HTMLElement;

  // Jika sedang di mode full, jangan izinkan menutup
  if (island.classList.contains('mode-full')) return;

  if (island.classList.contains('collapsed')) {
    island.classList.replace('collapsed', 'expanded');
    compactView.style.display = 'none';
    expandedView.style.display = 'flex';
  } else {
    island.classList.replace('expanded', 'collapsed');
    expandedView.style.display = 'none';
    compactView.style.display = 'flex';
  }
}

(window as any).toggleIsland = toggleIsland;

// --- 2. Click-Outside to Close ---
appWindow.onFocusChanged(({ payload: focused }) => {
  if (!focused && island.classList.contains('expanded')) {
    toggleIsland();
  }
}).catch((error) => {
  console.error('Gagal memasang listener fokus jendela:', error);
});

// --- 3. Logika Ganti Mode ---
(window as any).changeDisplayMode = (mode: string) => {
  // Hapus class mode sebelumnya
  island.classList.remove('mode-minimal', 'mode-full', 'mode-hover');
  
  // Reset state ke collapsed (kecil)
  island.classList.remove('expanded');
  island.classList.add('collapsed');
  (document.getElementById('expanded-view') as HTMLElement).style.display = 'none';
  (document.getElementById('compact-view') as HTMLElement).style.display = 'flex';

  // Terapkan perilaku tampilan
  switch (mode) {
    case 'hover':
      // Mode Hover: kapsul disembunyikan oleh CSS saat collapsed
      // dan muncul kembali ketika area jendela di-hover.
      island.classList.add('mode-hover');
      break;
      
    case 'minimal':
      island.classList.add('mode-minimal');
      break;

    case 'full':
      island.classList.add('mode-full');
      // Paksa buka tampilan besar
      island.classList.replace('collapsed', 'expanded');
      (document.getElementById('compact-view') as HTMLElement).style.display = 'none';
      (document.getElementById('expanded-view') as HTMLElement).style.display = 'flex';
      break;

    case 'fixed':
    default:
      // Mode Default: selalu terlihat dan bisa diklik
      break;
  }
};

// --- 4. Logika Tab Navigation ---
const tabButtons = document.querySelectorAll<HTMLButtonElement>('.tab-btn');
const tabContents = document.querySelectorAll<HTMLElement>('.tab-content');

tabButtons.forEach((button) => {
  button.addEventListener('click', () => {
    const targetId = button.dataset.target;
    if (!targetId) return;

    tabButtons.forEach((tab) => tab.classList.remove('active'));
    tabContents.forEach((content) => content.classList.remove('active'));

    button.classList.add('active');
    document.getElementById(targetId)?.classList.add('active');
  });
});

// Jam (Sama seperti sebelumnya)
function updateClock() {
  const now = new Date();
  const timeString = now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const compactTime = document.getElementById('compact-time');
  const expandedTime = document.getElementById('expanded-time');
  if (compactTime) compactTime.innerText = timeString;
  if (expandedTime) expandedTime.innerText = timeString;
}
setInterval(updateClock, 1000);
updateClock();

// --- 5. Info Sistem (CPU, RAM, Baterai, Suhu) ---
type SystemStats = {
  cpuUsage: number;
  ramUsageGb: number;
  batteryPercent: number | null;
};

function setText(id: string, text: string) {
  const element = document.getElementById(id);
  if (element) element.innerText = text;
}

async function updateSystemStats() {
  const [statsResult, tempResult] = await Promise.allSettled([
    invoke<SystemStats>('get_system_stats'),
    invoke<number>('get_cpu_temperature'),
  ]);

  if (statsResult.status === 'fulfilled') {
    const stats = statsResult.value;
    const batteryText = stats.batteryPercent === null
      ? '--'
      : `${Math.round(stats.batteryPercent)}%`;

    setText('sys-cpu', `${Math.round(stats.cpuUsage)}%`);
    setText('sys-ram', `${stats.ramUsageGb.toFixed(1)} GB`);
    setText('compact-battery', batteryText);
    setText('expanded-battery', batteryText);
  } else {
    console.error('Gagal mengambil informasi sistem:', statsResult.reason);
  }

  if (tempResult.status === 'fulfilled') {
    const tempText = `${Math.round(tempResult.value)}°C`;
    setText('mini-temp', tempText);
    setText('sys-temp', tempText);
  } else {
    console.error('Gagal mengambil suhu CPU:', tempResult.reason);
    setText('mini-temp', '--');
    setText('sys-temp', '--');
  }
}

setInterval(updateSystemStats, 3000);
updateSystemStats();

// --- 6. Status Wi-Fi & Bluetooth ---
type NetworkStatus = {
  wifiConnected: boolean;
  wifiName: string;
  btConnected: boolean;
  btName: string;
};

function updateNetworkItem(
  itemId: string,
  statusId: string,
  connected: boolean,
  name: string,
) {
  const item = document.getElementById(itemId);
  const status = document.getElementById(statusId);
  if (!item || !status) return;

  const label = connected ? (name?.trim() || 'Connected') : 'Disconnected';

  item.classList.toggle('active', connected);
  status.innerText = label;
  status.title = label;
}

let networkPollInFlight = false;

async function updateNetworkStatus() {
  // Avoid piling up PowerShell calls if the previous poll is still running.
  if (networkPollInFlight) return;
  networkPollInFlight = true;

  try {
    const status = await invoke<NetworkStatus>('get_network_status');
    updateNetworkItem(
      'wifi-status-item',
      'wifi-status',
      status.wifiConnected,
      status.wifiName,
    );
    updateNetworkItem(
      'bluetooth-status-item',
      'bluetooth-status',
      status.btConnected,
      status.btName,
    );
  } catch (error) {
    console.error('Gagal mengambil status jaringan:', error);
    updateNetworkItem('wifi-status-item', 'wifi-status', false, '');
    updateNetworkItem('bluetooth-status-item', 'bluetooth-status', false, '');
  } finally {
    networkPollInFlight = false;
  }
}

setInterval(updateNetworkStatus, 5000);
updateNetworkStatus();

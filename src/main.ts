import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();
const island = document.getElementById('dynamic-island') as HTMLElement;

// --- 1. Logika Buka/Tutup (Sama seperti sebelumnya) ---
(window as any).toggleIsland = () => {
  const compactView = document.getElementById('compact-view') as HTMLElement;
  const expandedView = document.getElementById('expanded-view') as HTMLElement;

  // Jika sedang di mode full, jangan izinkan menutup
  if (island.classList.contains('mode-full')) return;

  if (island.classList.contains('collapsed')) {
    island.classList.replace('collapsed', 'expanded');
    compactView.style.display = 'none';
    expandedView.style.display = 'block';
  } else {
    island.classList.replace('expanded', 'collapsed');
    expandedView.style.display = 'none';
    compactView.style.display = 'flex';
  }
};

// --- 2. Logika Ganti Mode ---
(window as any).changeDisplayMode = async (mode: string) => {
  // Hapus class mode sebelumnya
  island.classList.remove('mode-minimal', 'mode-full', 'mode-hover');
  
  // Reset state ke collapsed (kecil)
  island.classList.remove('expanded');
  island.classList.add('collapsed');
  (document.getElementById('expanded-view') as HTMLElement).style.display = 'none';
  (document.getElementById('compact-view') as HTMLElement).style.display = 'flex';

  // Terapkan perilaku jendela Tauri
  switch (mode) {
    case 'hover':
      // Mode Hover: Jendela tembus klik (tidak menghalangi aplikasi lain)
      // Kapsul akan membesar saat mouse diarahkan ke atasnya
      await appWindow.setIgnoreCursorEvents(true);
      island.classList.add('mode-hover');
      break;
      
    case 'minimal':
      await appWindow.setIgnoreCursorEvents(false);
      island.classList.add('mode-minimal');
      break;

    case 'full':
      await appWindow.setIgnoreCursorEvents(false);
      island.classList.add('mode-full');
      // Paksa buka tampilan besar
      island.classList.replace('collapsed', 'expanded');
      (document.getElementById('compact-view') as HTMLElement).style.display = 'none';
      (document.getElementById('expanded-view') as HTMLElement).style.display = 'block';
      break;

    case 'fixed':
    default:
      // Mode Default: Selalu terlihat, bisa di-klik
      await appWindow.setIgnoreCursorEvents(false);
      break;
  }
};

// --- 3. Logika Khusus Mode Hover ---
// Saat mouse masuk ke area kapsul, nyalakan interaksi klik
island.addEventListener('mouseenter', async () => {
  if (island.classList.contains('mode-hover')) {
    await appWindow.setIgnoreCursorEvents(false);
  }
});

// Saat mouse keluar, matikan interaksi klik agar tembus pandang
island.addEventListener('mouseleave', async () => {
  if (island.classList.contains('mode-hover')) {
    // Jika sedang expand, jangan sembunyikan dulu (opsional)
    if (island.classList.contains('collapsed')) {
      await appWindow.setIgnoreCursorEvents(true);
    }
  }
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
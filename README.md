# StreamAudio

> **AI Notu / AI Note:** Bu proje yapay zekâ (AI) desteğiyle geliştirilmiştir. / Built with AI assistance.

Windows 10/11 için sistem tepsisinden anında açılan, uygulama bazlı canlı ses mikseri, tek kısayolla **Kulaklık ↔ Hoparlör** geçişi ve oyun içi uygulama susturma/kısma sunan açık kaynaklı, sürücüsüz (%100 WASAPI) modern ses kontrol merkezi.

Open-source, driverless (100% WASAPI) modern audio control center for Windows 10/11 featuring an instant system tray mixer, live per-app peak meters, one-hotkey **Headphones ↔ Speakers** switching, and in-game per-app mute/duck controls.

---

## Özellikler / Features

### Türkçe
- **Uygulama Bazlı Canlı Ses Mikseri:** Ses çıkaran tüm açık uygulamaları (oyunlar, Discord, Spotify, Chrome, OBS vb.) gerçek zamanlı listeler; her uygulama için %0–100 ses kaydırıcısı, tek tıkla Sustur (Mute), hızlı %20 Kıs (Duck) ve `IAudioMeterInformation` destekli canlı **ses seviyesi (peak meter)** barı sunar.
- **Anında Çıkış Cihazı Değiştirme (Kulaklık ↔ Hoparlör):** Tüm aktif ses çıkış cihazlarını (`IMMDeviceEnumerator`) ve ana ses seviyesini listeler; tek tıkla varsayılan çıkış cihazını (`IPolicyConfig::SetDefaultEndpoint`) değiştirir.
- **Oyun İçi Global Kısayollar + OSD Bildirimi:**
  - `Ctrl+Alt+S` (özelleştirilebilir): Seçtiğiniz 2 favori cihaz (ör. Kulaklık ↔ Hoparlör) arasında alt-tab atmadan tek tuşla geçiş yapar ve şeffaf OSD bildirimi gösterir.
  - `Ctrl+Alt+M` (özelleştirilebilir): Varsayılan mikrofonu tek tuşla susturur/açar.
  - **Uygulama Kısayolları (Quick Mute & Duck):** Spotify, Chrome vb. uygulamalara özel global kısayol atayarak tek tuşla susturabilir veya sesini %20'ye çekip geri yükseltebilirsiniz.
- **EarTrumpet Tarzı Sistem Tepsisi Deneyimi:** Sistem tepsisi ikonuna tıklandığında görev çubuğunun hemen üstünde kompakt pencere olarak açılır, dışarı tıklandığında veya `Esc` ile gizlenir; istenirse normal pencere moduna sabitlenebilir.
- **TR / EN Dil Desteği:** Üst çubuktan anlık Türkçe / İngilizce geçişi.

### English
- **Live Per-App Audio Mixer:** Real-time list of all active audio sessions (games, Discord, Spotify, Chrome, OBS, etc.) with 0–100% volume sliders, one-click Mute, quick 20% Duck toggle, and live **peak meter bars** powered by `IAudioMeterInformation`.
- **Instant Output Device Switcher (Headphones ↔ Speakers):** Lists all active playback endpoints (`IMMDeviceEnumerator`) with individual volume controls and one-click default device switching (`IPolicyConfig::SetDefaultEndpoint`).
- **In-Game Global Hotkeys + Transparent OSD:**
  - `Ctrl+Alt+S` (customizable): Instantly switches between your 2 favorite output devices without alt-tabbing and displays a sleek non-intrusive OSD toast.
  - `Ctrl+Alt+M` (customizable): Instant global microphone mute toggle.
  - **Per-App Quick Mute & Duck:** Bind custom hotkeys to specific apps (e.g., `spotify.exe`, `chrome.exe`) to toggle mute or duck volume to 20% and restore it back.
- **EarTrumpet-Style System Tray Flyout:** Opens cleanly right above the taskbar from the system tray, auto-hides on focus loss or `Esc`, and supports pinning to normal window mode.
- **Live TR / EN Switcher:** Instant Turkish / English UI language switching.

---

## Teknoloji Yığını / Tech Stack

- **Backend:** Rust + Tauri v2 + `windows` (Win32 Core Audio / WASAPI: `IMMDeviceEnumerator`, `IAudioSessionManager2`, `ISimpleAudioVolume`, `IAudioMeterInformation`, `IPolicyConfig`) + `tauri-plugin-global-shortcut`
- **Frontend:** React 19 + TypeScript + Vite + Tailwind CSS + Lucide Icons

---

## Geliştirme ve Derleme / Development & Build

```bash
# Bağımlılıkları yükle / Install dependencies
npm install

# Geliştirme modunda çalıştır / Run in dev mode
npm run tauri dev

# Release derlemesi al / Build release binary
npm run tauri build
```

---

*Bu proje yapay zekâ (AI) desteğiyle geliştirilmiştir. / Built with AI assistance.*

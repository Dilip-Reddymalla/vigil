//! Active-audio presence via WASAPI (Phase 1 / audio shield).
//!
//! If any audio is actively playing through the default render endpoint, the
//! engine should not drop the user to Idle while they're watching/listening
//! without keyboard/mouse input. Best-effort: any failure returns `false` so the
//! service never breaks if COM/audio is unavailable.

use windows::core::GUID;
use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
use windows::Win32::Media::Audio::{EDataFlow, ERole, IMMDeviceEnumerator};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

/// `CLSID_MMDeviceEnumerator`
const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID::from_u128(0xBCDE0395_E52F_467C_8E3D_C4579291692E);

/// Render endpoint; console role.
const E_RENDER: EDataFlow = EDataFlow(0);
const E_CONSOLE: ERole = ERole(0);

const AUDIO_PEAK_THRESHOLD: f32 = 0.001;

/// True when audio is actively playing through the default output device.
pub fn audio_playing() -> bool {
    // SAFETY: COM init; all interfaces are released on drop. Any failure → false.
    unsafe {
        let _ = CoInitializeEx(Some(std::ptr::null()), COINIT_MULTITHREADED);

        let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
            &CLSID_MMDEVICE_ENUMERATOR,
            None,
            CLSCTX_ALL,
        ) else {
            return false;
        };
        let Ok(device) = enumerator.GetDefaultAudioEndpoint(E_RENDER, E_CONSOLE) else {
            return false;
        };
        let Ok(meter) = device.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None) else {
            return false;
        };
        meter.GetPeakValue().map(|peak| peak > AUDIO_PEAK_THRESHOLD).unwrap_or(false)
    }
}

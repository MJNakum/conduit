//! Vault lock: verify the device owner via Touch ID / macOS password using the
//! LocalAuthentication framework. The webview calls `vault_authenticate` when
//! the user asks to unlock; on success it reveals the app again. No secret is
//! read or moved here — this only gates the UI.

#[cfg(target_os = "macos")]
#[tauri::command]
pub async fn vault_authenticate(reason: String) -> Result<bool, String> {
    // The LocalAuthentication reply arrives on a private queue; block a pooled
    // thread on a channel so the async command resolves when it fires.
    tokio::task::spawn_blocking(move || evaluate(&reason))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(target_os = "macos")]
fn evaluate(reason: &str) -> Result<bool, String> {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_local_authentication::{LAContext, LAPolicy};
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel::<bool>();
    let reason = NSString::from_str(reason);
    // DeviceOwnerAuthentication allows the macOS password as a fallback when
    // Touch ID is unavailable or fails — matching the user's chosen behavior.
    let policy = LAPolicy::DeviceOwnerAuthentication;

    let reply = RcBlock::new(move |success: Bool, _error: *mut NSError| {
        let _ = tx.send(success.as_bool());
    });

    unsafe {
        let ctx = LAContext::new();
        ctx.evaluatePolicy_localizedReason_reply(policy, &reason, &reply);
    }

    // If the framework never calls back (shouldn't happen), treat as failure.
    rx.recv().map_err(|_| "authentication cancelled".to_string())
}

// Windows: gate via Windows Hello (UserConsentVerifier). An unpackaged Win32 app
// has no CoreWindow, so we must go through the HWND interop and parent the prompt
// to our own window rather than calling the plain WinRT static.
#[cfg(target_os = "windows")]
#[tauri::command]
pub async fn vault_authenticate(
    window: tauri::WebviewWindow,
    reason: String,
) -> Result<bool, String> {
    use windows::core::{Interface, HSTRING};
    use windows::Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier,
    };
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::WinRT::IUserConsentVerifierInterop;

    // Reconstruct HWND from the raw pointer so a windows-crate version skew with
    // Tauri's own dependency can't cause a type mismatch.
    let raw = window.hwnd().map_err(|e| e.to_string())?;
    let hwnd = HWND(raw.0 as _);
    let message = HSTRING::from(reason);

    // The async result arrives off-thread; block a pooled thread so the async
    // command resolves when it fires (mirrors the macOS path).
    tokio::task::spawn_blocking(move || -> Result<bool, String> {
        let interop = UserConsentVerifier::factory::<IUserConsentVerifierInterop>()
            .map_err(|e| e.to_string())?;
        let op = unsafe {
            interop.RequestVerificationForWindowAsync(hwnd, &message)
        }
        .map_err(|e| e.to_string())?;
        let result: UserConsentVerificationResult = op.get().map_err(|e| e.to_string())?;
        Ok(result == UserConsentVerificationResult::Verified)
    })
    .await
    .map_err(|e| e.to_string())?
}

// Other platforms (Linux): no biometric gate yet — unlock succeeds so the app
// stays usable.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[tauri::command]
pub async fn vault_authenticate(_reason: String) -> Result<bool, String> {
    Ok(true)
}

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

// Non-macOS: no biometric gate yet — unlock succeeds so the app stays usable.
#[cfg(not(target_os = "macos"))]
#[tauri::command]
pub async fn vault_authenticate(_reason: String) -> Result<bool, String> {
    Ok(true)
}

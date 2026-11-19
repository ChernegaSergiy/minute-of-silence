//! Reports overlay readiness to the Android activity.

use crate::error::Result;
use crate::state::{CeremonyKind, CeremonyPhase, CeremonyRuntime};
use serde_json::json;

/// Tells Android that the webview has drawn the overlay content.
pub async fn mark_ready(runtime: &CeremonyRuntime, ceremony_id: u64) -> Result<()> {
    let should_update = {
        let inner = runtime.lock();
        inner.settings.show_visual_overlay
            && inner.active_ceremony.is_some_and(|active| {
                active.id == ceremony_id
                    && active.kind == CeremonyKind::Scheduled
                    && active.phase != CeremonyPhase::Finishing
            })
    };
    if should_update {
        super::device::call::<()>(
            "markCeremonyOverlayReady",
            json!({ "ceremonyId": ceremony_id.to_string() }),
        )?;
    }
    Ok(())
}

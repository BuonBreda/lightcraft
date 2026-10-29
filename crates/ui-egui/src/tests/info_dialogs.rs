//! #782: dialogs that only show something (System Info, All Metadata, What's New) end with one
//! Close, like About, instead of a Cancel / OK pair that both just close them.

use crate::headless::Headless;
use crate::{LightcraftApp, Services};
use serde_json::json;
use std::time::Duration;

const T: Duration = Duration::from_secs(20);
const SETTLE: Duration = Duration::from_secs(60);

fn has(h: &Headless, id: &str) -> bool {
    h.app.widgets.iter().any(|(w, _)| w == id)
}

/// Open the dialog through its menu item; let the new window size itself before reading it.
fn open(h: &mut Headless, id: &str) {
    let r = h.request("ui.menu.invoke", json!({"id": id}), T);
    assert_eq!(r["ok"], true, "{id}: {r}");
    assert!(h.app.ui.dialog.is_some(), "{id}: no dialog");
    h.step();
    h.step();
}

#[test]
fn information_dialogs_end_with_one_close() {
    let app = LightcraftApp::new(lightcraft_engine::Session::with_demo(), Services { png: None, ..Default::default() });
    let mut h = Headless::new(app, [1300.0, 900.0], 1.0);
    h.settle(SETTLE);
    for id in ["app.systemInfo", "dialog.allMetadata", "app.whatsNew"] {
        open(&mut h, id);
        assert!(!has(&h, "button:dialogCancel"), "{id}: a Cancel button, though there is nothing to confirm");
        assert!(has(&h, "button:dialogOk"), "{id}: no Close button");
        let close = h.app.widgets.iter().filter(|(w, _)| w == "button:dialogOk").count();
        assert_eq!(close, 1, "{id}: one footer button");
        let text = h.painted_text();
        assert!(text.iter().any(|t| t == "Close"), "{id}: the button reads Close: {text:?}");
        assert!(!text.iter().any(|t| t == "Cancel" || t == "OK"), "{id}: no Cancel / OK: {text:?}");
        let r = h.request("ui.clickWidget", json!({"id": "button:dialogOk"}), T);
        assert_eq!(r["ok"], true, "{id}: {r}");
        h.step();
        assert!(h.app.ui.dialog.is_none(), "{id}: Close closes the dialog");
        // Esc still closes it too
        open(&mut h, id);
        let r = h.request("ui.key", json!({"key": "Escape"}), T);
        assert_eq!(r["ok"], true, "{id}: {r}");
        h.step();
        assert!(h.app.ui.dialog.is_none(), "{id}: Esc closes the dialog");
    }
}

/// A JPEG on disk with the camera make and ISO in its EXIF.
fn tagged_jpeg(path: &std::path::Path, iso: u32) {
    let img = lightcraft_raster::Rgba8 { width: 16, height: 8, data: vec![[90, 120, 200, 255]; 128] };
    let exif = lightcraft_meta::write_exif(&lightcraft_meta::Metadata { make: Some("Maker".into()), iso: Some(iso), ..Default::default() });
    let meta = lightcraft_codecs::EncodeMeta { exif: Some(&exif), ..Default::default() };
    let jpg = lightcraft_codecs::encode_jpeg(&lightcraft_codecs::EncodeImage::rgba8(&img), 90, Default::default(), &meta).unwrap();
    std::fs::write(path, jpg).unwrap();
}

/// Gap 30: with several photos selected and Show Metadata For on Selected Photos, All Metadata
/// shows what the selected photos share and says Mixed where they differ; on Target Photo it
/// shows the target photo alone, as the Info panel does.
#[test]
fn all_metadata_of_selected_photos_says_mixed() {
    let dir = std::env::temp_dir().join(format!("lc-ui-allmeta-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (a, b) = (dir.join("a.jpg"), dir.join("b.jpg"));
    tagged_jpeg(&a, 800);
    tagged_jpeg(&b, 1600);
    let mut session = lightcraft_engine::Session::new().with_fs();
    session.execute("library.import", &json!({"paths": [a.to_string_lossy(), b.to_string_lossy()]})).unwrap();
    session.execute("library.selectAll", &json!({})).unwrap();
    assert_eq!(session.selected().len(), 2);
    let app = LightcraftApp::new(session, Services { png: None, ..Default::default() });
    let mut h = Headless::new(app, [1300.0, 900.0], 1.0);
    h.settle(SETTLE);

    h.app.ui.settings.info_selected_photos = true;
    open(&mut h, "dialog.allMetadata");
    let text = h.painted_text();
    assert!(text.iter().any(|t| t == "2 selected photos"), "the title counts the photos: {text:?}");
    assert!(text.iter().any(|t| t == "Maker"), "a shared value is shown: {text:?}");
    assert!(text.iter().any(|t| t == "Mixed"), "the ISO they differ in says Mixed: {text:?}");
    assert!(!text.iter().any(|t| t == "800" || t == "1600"), "no one photo's ISO stands for both: {text:?}");
    h.app.ui.dialog = None;
    h.step();

    h.app.ui.settings.info_selected_photos = false;
    open(&mut h, "dialog.allMetadata");
    let text = h.painted_text();
    assert!(!text.iter().any(|t| t == "Mixed" || t == "2 selected photos"), "Target Photo shows one photo: {text:?}");
    assert!(text.iter().any(|t| t == "800" || t == "1600"), "its own ISO: {text:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

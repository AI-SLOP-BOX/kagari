use super::*;

#[test]
fn test_reload_ame_ui_runtime() {
    let mut app = AfterEffectsStudio::new().unwrap();
    invoke(&mut app, "back_to_editor", None);
    assert_eq!(app.screen, Screen::Editor);

    // Invoke reload_ame_ui
    invoke(&mut app, "reload_ame_ui", None);
    assert_eq!(app.asset_status, "AmeUI ホットリロード成功");
    assert_eq!(app.screen, Screen::Editor);
}

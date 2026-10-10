#[cfg(test)]
mod tests {
    use crate::app_state::QueueItemStatus;
    use crate::ui::export_dialog::*;
    use eframe::egui;

    #[test]
    fn work_area_export_range_includes_both_endpoints() {
        assert_eq!(export_frame_range(100, 1, Some(10), Some(20)), (10, 11));
        assert_eq!(export_frame_range(100, 1, Some(10), None), (10, 90));
        assert_eq!(export_frame_range(100, 1, None, Some(20)), (0, 21));
    }

    #[test]
    fn export_range_clamps_invalid_work_area_without_underflow() {
        assert_eq!(export_frame_range(10, 1, Some(50), Some(80)), (9, 1));
        assert_eq!(export_frame_range(10, 1, Some(8), Some(3)), (8, 1));
        assert_eq!(export_frame_range(0, 1, Some(0), Some(0)), (0, 0));
    }

    #[test]
    fn batch_output_paths_are_distinct_and_keep_the_output_directory() {
        let first = batch_output_path("/tmp/final.mov", "Main Comp", "comp_1");
        let second = batch_output_path("/tmp/final.mov", "Main/Comp", "comp_2");
        assert_eq!(
            std::path::Path::new(&first).parent().unwrap(),
            std::path::Path::new("/tmp")
        );
        assert!(first.ends_with(".mov"));
        assert_ne!(first, second);
        assert!(!second.contains("Main/Comp"));
    }

    fn drive_events(app: &mut crate::KagariApp) {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            draw(app, ctx);
        });
    }

    #[test]
    fn finished_event_marks_queue_item_done_and_advances_batch() {
        let mut app = crate::KagariApp::default();
        let comp_id = app.history.current().active_composition().id.clone();
        app.render_queue_items = vec![comp_id.clone()];
        app.batch_queue = vec![comp_id.clone()];
        app.batch_idx = 0;
        app.render_item_status
            .insert(comp_id.clone(), QueueItemStatus::Rendering);
        app.export.is_exporting = true;
        app.export.export_comp_id = Some(comp_id.clone());
        app.export.export_comp_name = Some(app.history.current().active_composition().name.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(crate::ExportEvent::Finished("ok".into())).unwrap();
        app.export.export_rx = Some(rx);

        drive_events(&mut app);

        assert_eq!(
            app.render_item_status.get(&comp_id),
            Some(&QueueItemStatus::Done)
        );
        assert!(!app.export.is_exporting);
        assert!(app.batch_queue.is_empty());
    }

    #[test]
    fn error_event_marks_queue_item_failed_and_aborts_batch() {
        let mut app = crate::KagariApp::default();
        let comp_id = app.history.current().active_composition().id.clone();
        app.render_queue_items = vec![comp_id.clone(), "missing-id".into()];
        app.batch_queue = vec![comp_id.clone(), "missing-id".into()];
        app.batch_idx = 0;
        app.export.is_exporting = true;
        app.export.export_comp_id = Some(comp_id.clone());
        app.export.export_comp_name = Some(app.history.current().active_composition().name.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(crate::ExportEvent::Error("boom".into())).unwrap();
        app.export.export_rx = Some(rx);

        drive_events(&mut app);

        assert_eq!(
            app.render_item_status.get(&comp_id),
            Some(&QueueItemStatus::Failed)
        );
        assert!(app.batch_queue.is_empty());
    }
}

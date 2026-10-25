use crate::core::timeline::EffectType;
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    _current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) {
    if let EffectType::ChannelCombiner {
        from_channel,
        to_target,
        invert,
    } = effect_type
    {
        let from_options = [
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Red,
                "Red",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Green,
                "Green",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Blue,
                "Blue",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Alpha,
                "Alpha",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Luminance,
                "Luminance",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Hue,
                "Hue",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Lightness,
                "Lightness",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::Saturation,
                "Saturation",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::MinRGB,
                "Min RGB",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerFrom::MaxRGB,
                "Max RGB",
            ),
        ];
        let to_options = [
            (crate::core::channel_combiner::ChannelCombinerTo::Red, "Red"),
            (
                crate::core::channel_combiner::ChannelCombinerTo::Green,
                "Green",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerTo::Blue,
                "Blue",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerTo::Alpha,
                "Alpha",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerTo::RGBOnly,
                "RGB Only",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerTo::RGBA,
                "RGBA",
            ),
            (
                crate::core::channel_combiner::ChannelCombinerTo::Lightness,
                "Lightness",
            ),
        ];
        let grid = crate::ui::kui::PropertyGrid::new(ui, &["From", "To", "Invert"]);
        *project_changed |= grid.row(ui, "From", |ui| {
            crate::ui::kui::combo(ui, "chan_comb_from", from_channel, &from_options)
        });
        *project_changed |= grid.row(ui, "To", |ui| {
            crate::ui::kui::combo(ui, "chan_comb_to", to_target, &to_options)
        });
        *project_changed |= grid
            .row(ui, "Invert", |ui| crate::ui::kui::switch(ui, invert, ""))
            .changed();
    }
}

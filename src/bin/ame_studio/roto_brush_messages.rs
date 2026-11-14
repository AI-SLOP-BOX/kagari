pub(crate) fn localize_propagation_error(error: &str) -> String {
    let Some((frame, reason)) = error
        .strip_prefix("Roto Brush correction at frame ")
        .and_then(|rest| rest.split_once(' '))
    else {
        return error.to_owned();
    };
    let Ok(frame) = frame.parse::<u32>() else {
        return error.to_owned();
    };
    match reason {
        "could not be segmented" => {
            format!(
                "フレーム {frame} のロト修正を解析できませんでした。ストロークを見直してください"
            )
        }
        "could not be applied safely" => {
            format!("フレーム {frame} のロト修正は安全条件を満たさず適用できませんでした")
        }
        _ => error.to_owned(),
    }
}

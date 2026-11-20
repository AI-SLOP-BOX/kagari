use super::*;

impl AfterEffectsStudio {
    pub(crate) fn sync_recent_projects_bindings(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let has_recents = !self.recent_projects.is_empty();
        self.ui
            .set_binding("has_recents", Value::Bool(has_recents))?;
        let mut activity_items: Vec<_> = self.recent_projects.iter().collect();
        activity_items.sort_by(|a, b| match (a.date == "—", b.date == "—") {
            (true, true) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => b.date.cmp(&a.date),
        });
        let recent_project_items = activity_items
            .into_iter()
            .map(|item| {
                Value::Record(std::collections::BTreeMap::from([
                    (
                        "path".to_owned(),
                        Value::String(item.path.display().to_string()),
                    ),
                    ("name".to_owned(), Value::String(item.name.clone())),
                    ("info".to_owned(), Value::String(item.comps_info.clone())),
                    (
                        "specs".to_owned(),
                        Value::String(format!(
                            "{} · {} · {}",
                            item.resolution, item.fps, item.duration
                        )),
                    ),
                    (
                        "location".to_owned(),
                        Value::String(item.path_label.clone()),
                    ),
                    ("date".to_owned(), Value::String(item.date.clone())),
                    ("selected".to_owned(), Value::Bool(false)),
                ]))
            })
            .collect::<Vec<_>>();
        self.ui
            .set_binding("recent_project_items", Value::List(recent_project_items))?;

        let query = self.project_search.trim().to_lowercase();
        let mut filtered_projects = self
            .recent_projects
            .iter()
            .filter(|item| {
                query.is_empty()
                    || item.name.to_lowercase().contains(&query)
                    || item.path_label.to_lowercase().contains(&query)
                    || item.path.to_string_lossy().to_lowercase().contains(&query)
            })
            .cloned()
            .collect::<Vec<_>>();
        filtered_projects.sort_by(|a, b| {
            let ord = match self.project_sort.as_str() {
                "name" => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                "created" => {
                    let ca = std::fs::metadata(&a.path)
                        .and_then(|m| m.created())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    let cb = std::fs::metadata(&b.path)
                        .and_then(|m| m.created())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    ca.cmp(&cb).then_with(|| a.date.cmp(&b.date))
                }
                _ => a.date.cmp(&b.date),
            };
            if self.project_sort_desc {
                ord.reverse()
            } else {
                ord
            }
        });
        let project_items = filtered_projects
            .iter()
            .map(|item| {
                let path_str = item.path.display().to_string();
                let selected = self.selected_project_paths.contains(&path_str);
                Value::Record(std::collections::BTreeMap::from([
                    ("path".to_owned(), Value::String(path_str)),
                    ("name".to_owned(), Value::String(item.name.clone())),
                    ("info".to_owned(), Value::String(item.comps_info.clone())),
                    (
                        "specs".to_owned(),
                        Value::String(format!(
                            "{} · {} · {} · {}レイヤー",
                            item.resolution, item.fps, item.duration, item.layer_count
                        )),
                    ),
                    (
                        "location".to_owned(),
                        Value::String(item.path_label.clone()),
                    ),
                    ("date".to_owned(), Value::String(item.date.clone())),
                    ("selected".to_owned(), Value::Bool(selected)),
                ]))
            })
            .collect::<Vec<_>>();
        let project_count_label = format!("{}件のプロジェクト", project_items.len());
        let selected_count = self.selected_project_paths.len();
        self.ui
            .set_binding("project_search", Value::String(self.project_search.clone()))?;
        self.ui
            .set_binding("project_sort", Value::String(self.project_sort.clone()))?;
        self.ui
            .set_binding("project_sort_desc", Value::Bool(self.project_sort_desc))?;
        self.ui
            .set_binding("project_view_grid", Value::Bool(self.project_view_grid))?;
        self.ui
            .set_binding("project_has_selection", Value::Bool(selected_count > 0))?;
        self.ui.set_binding(
            "project_selection_label",
            Value::String(format!("{selected_count}件選択中")),
        )?;
        self.ui.set_binding(
            "has_project_matches",
            Value::Bool(!project_items.is_empty()),
        )?;
        self.ui
            .set_binding("project_count_label", Value::String(project_count_label))?;
        self.ui
            .set_binding("project_items", Value::List(project_items))?;

        for i in 1..=8 {
            let active = i <= self.recent_projects.len();
            let has_key = format!("has_recent_{i}");
            self.ui.set_binding(&has_key, Value::Bool(active))?;

            if let Some(item) = self.recent_projects.get(i - 1) {
                self.ui.set_binding(
                    &format!("recent_{i}_name"),
                    Value::String(item.name.clone()),
                )?;
                self.ui.set_binding(
                    &format!("recent_{i}_path"),
                    Value::String(item.path_label.clone()),
                )?;
                if i == 1 {
                    self.ui
                        .set_binding("recent_1_path", Value::String(item.path_label.clone()))?;
                    self.ui.set_binding(
                        "recent_1_layers",
                        Value::String(item.layer_count.to_string()),
                    )?;
                    self.ui.set_binding(
                        "recent_1_day",
                        Value::String(
                            item.date
                                .split_whitespace()
                                .next()
                                .unwrap_or_default()
                                .to_owned(),
                        ),
                    )?;
                }
                self.ui.set_binding(
                    &format!("recent_{i}_date"),
                    Value::String(item.date.clone()),
                )?;
                self.ui.set_binding(
                    &format!("recent_{i}_len"),
                    Value::String(item.duration.clone()),
                )?;
                self.ui
                    .set_binding(&format!("recent_{i}_fps"), Value::String(item.fps.clone()))?;
                self.ui.set_binding(
                    &format!("recent_{i}_res"),
                    Value::String(item.resolution.clone()),
                )?;
                self.ui.set_binding(
                    &format!("recent_{i}_info"),
                    Value::String(item.comps_info.clone()),
                )?;
            } else {
                self.ui
                    .set_binding(&format!("recent_{i}_name"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_path"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_date"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_len"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_fps"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_res"), Value::String(String::new()))?;
                self.ui
                    .set_binding(&format!("recent_{i}_info"), Value::String(String::new()))?;
            }
        }
        Ok(())
    }
}

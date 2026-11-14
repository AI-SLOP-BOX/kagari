use super::*;

struct ProxyWorkspaceGuard {
    path: std::path::PathBuf,
    keep: bool,
}

impl Drop for ProxyWorkspaceGuard {
    fn drop(&mut self) {
        if self.keep {
            return;
        }
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "could not remove proxy workspace {}: {error}",
                    self.path.display()
                );
            }
        }
    }
}

impl AfterEffectsStudio {
    pub(crate) fn cycle_selected_layer_proxy_resolution(&mut self) {
        use kagari_vfx::core::proxy::ProxyResolution;
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        layer.proxy.resolution = match layer.proxy.resolution {
            ProxyResolution::Full => ProxyResolution::Half,
            ProxyResolution::Half => ProxyResolution::Quarter,
            ProxyResolution::Quarter => ProxyResolution::Eighth,
            ProxyResolution::Eighth => ProxyResolution::Full,
        };
        self.mark_project_changed();
    }

    pub(crate) fn toggle_selected_layer_proxy(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        layer.proxy.enabled = !layer.proxy.enabled;
        self.mark_project_changed();
    }

    pub(crate) fn clear_selected_layer_proxy(&mut self) {
        let Some(id) = self.selected_layer_id.clone() else {
            return;
        };
        let Some(layer) = self
            .project_model
            .active_composition_mut()
            .layers
            .iter_mut()
            .find(|layer| layer.id == id && !layer.locked)
        else {
            return;
        };
        if layer.proxy.proxy_path.take().is_some() {
            layer.proxy.enabled = false;
            layer.proxy.path_resolution = None;
            self.mark_project_changed();
        }
    }

    pub(crate) fn start_selected_layer_proxy_generation(&mut self) {
        use kagari_vfx::core::proxy::{generate_preview_proxy_for_layer, ProxyResolution};
        if self.proxy_generation.is_some() {
            return;
        }
        let Some(layer_id) = self.selected_layer_id.clone() else {
            self.proxy_status = "プロキシを生成するレイヤーを選択してください。".to_owned();
            return;
        };
        let composition = self.project_model.active_composition();
        let Some(layer) = composition
            .layers
            .iter()
            .find(|layer| layer.id == layer_id && !layer.locked)
            .cloned()
        else {
            return;
        };
        let resolution = layer.proxy.resolution;
        if !matches!(
            &layer.layer_type,
            kagari_vfx::core::timeline::LayerType::Image { .. }
                | kagari_vfx::core::timeline::LayerType::Video { .. }
        ) {
            self.proxy_status = "プロキシ生成は画像と動画レイヤーに対応しています。".to_owned();
            return;
        }
        if resolution == ProxyResolution::Full {
            self.proxy_status = "生成前に解像度を1/2、1/4、1/8から選んでください。".to_owned();
            return;
        }
        let composition_id = composition.id.clone();
        let configured_cache = if self.preview_cache_dir.trim().is_empty() {
            default_preview_cache_dir()
        } else {
            self.preview_cache_dir.clone()
        };
        let proxy_root = expand_home_path(configured_cache.trim()).join("LayerProxies");
        if let Err(error) = std::fs::create_dir_all(&proxy_root) {
            self.proxy_status = format!("プロキシ保存先を作成できません: {error}");
            return;
        }
        let destination = proxy_root.join(format!(
            "kagari_proxy_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|time| time.as_nanos())
                .unwrap_or_default(),
        ));
        #[cfg(unix)]
        let mut workspace_builder = std::fs::DirBuilder::new();
        #[cfg(not(unix))]
        let workspace_builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut workspace_builder, 0o700);
        if let Err(error) = workspace_builder.create(&destination) {
            self.proxy_status = format!("プロキシ用の一時フォルダーを作成できません: {error}");
            return;
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_cancel = cancel.clone();
        let spawn_cleanup_destination = destination.clone();
        let workspace_guard = ProxyWorkspaceGuard {
            path: destination.clone(),
            keep: false,
        };
        let worker = std::thread::Builder::new()
            .name("kagari-proxy-generation".to_owned())
            .spawn(move || {
                let mut workspace_guard = workspace_guard;
                let last_sent = std::cell::Cell::new(None::<std::time::Instant>);
                let progress_sender = sender.clone();
                let mut progress = move |done: u32, total: u32| {
                    let now = std::time::Instant::now();
                    if done >= total
                        || last_sent.get().is_none_or(|previous| {
                            now.duration_since(previous) >= std::time::Duration::from_millis(100)
                        })
                    {
                        last_sent.set(Some(now));
                        let _ = progress_sender
                            .send(LayerProxyGenerationMessage::Progress { done, total });
                    }
                };
                let result = generate_preview_proxy_for_layer(
                    &layer,
                    resolution,
                    &destination,
                    &worker_cancel,
                    &mut progress,
                );
                let generated_successfully = result.is_ok();
                let delivered = sender
                    .send(LayerProxyGenerationMessage::Finished(result))
                    .is_ok();
                workspace_guard.keep = generated_successfully && delivered;
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                let _ = std::fs::remove_dir(&spawn_cleanup_destination);
                self.proxy_status = format!("プロキシ生成を開始できませんでした: {error}");
                return;
            }
        };
        self.proxy_generation = Some(LayerProxyGenerationJob {
            worker: Some(worker),
            receiver,
            cancel,
            resolution,
            layer_id,
            composition_id,
            project_file_path: self.project_file_path.clone(),
            project_name: self.project_name.clone(),
            project_generation: self.project_generation,
        });
        self.proxy_progress = 0;
        self.proxy_status = format!("{} プロキシを生成しています…", resolution.label());
    }

    pub(crate) fn cancel_layer_proxy_generation(&mut self) {
        if let Some(job) = &self.proxy_generation {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.proxy_status = "プロキシ生成をキャンセルしています…".to_owned();
        }
    }

    pub(crate) fn receive_layer_proxy_generation(&mut self) -> bool {
        let Some(job) = self.proxy_generation.as_ref() else {
            return false;
        };
        let mut messages = Vec::new();
        let mut disconnected = false;
        loop {
            match job.receiver.try_recv() {
                Ok(message) => messages.push(message),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if messages.is_empty() && !disconnected {
            return false;
        }
        let mut changed = false;
        for message in messages {
            match message {
                LayerProxyGenerationMessage::Progress { done, total } => {
                    self.proxy_progress = if total == 0 {
                        0
                    } else {
                        (u64::from(done).saturating_mul(100) / u64::from(total)).min(100) as i64
                    };
                    self.proxy_status =
                        format!("プロキシを生成しています… {done}/{total} フレーム");
                    changed = true;
                }
                LayerProxyGenerationMessage::Finished(result) => {
                    let Some(job) = self.proxy_generation.take() else {
                        return changed;
                    };
                    if let Some(worker) = job.worker {
                        let _ = worker.join();
                    }
                    let cancelled = job.cancel.load(std::sync::atomic::Ordering::Relaxed);
                    let target_is_current = self.project_generation == job.project_generation
                        && self.project_name == job.project_name
                        && self.project_file_path == job.project_file_path
                        && self.project_model.active_composition().id == job.composition_id;
                    match result {
                        Ok(path) if !cancelled && target_is_current => {
                            let applied = self
                                .project_model
                                .active_composition_mut()
                                .layers
                                .iter_mut()
                                .find(|layer| layer.id == job.layer_id)
                                .map(|layer| {
                                    layer.proxy.proxy_path = Some(path.clone());
                                    layer.proxy.path_resolution = Some(job.resolution);
                                    layer.proxy.enabled = true;
                                })
                                .is_some();
                            if applied {
                                kagari_vfx::core::frame_cache::bump_version();
                                self.proxy_status =
                                    "プロキシを生成し、プレビューに適用しました。".to_owned();
                                self.proxy_progress = 100;
                                self.mark_project_changed();
                            } else {
                                remove_generated_proxy(&path);
                                self.proxy_status = "生成後に対象レイヤーがなくなったため、プロキシを適用しませんでした。".to_owned();
                            }
                        }
                        Ok(path) => {
                            remove_generated_proxy(&path);
                            self.proxy_status = if cancelled {
                                "プロキシ生成をキャンセルしました。"
                            } else {
                                "プロジェクトが切り替わったため、プロキシを適用しませんでした。"
                            }
                            .to_owned();
                        }
                        Err(error) if error == kagari_vfx::core::proxy::PROXY_CANCELLED => {
                            self.proxy_status = "プロキシ生成をキャンセルしました。".to_owned();
                        }
                        Err(error) => {
                            self.proxy_status = format!("プロキシを生成できませんでした: {error}")
                        }
                    }
                    changed = true;
                }
            }
        }
        if disconnected && self.proxy_generation.is_some() {
            if let Some(job) = self.proxy_generation.take() {
                if let Some(worker) = job.worker {
                    let _ = worker.join();
                }
            }
            self.proxy_status = "プロキシ生成ワーカーが停止しました。".to_owned();
            changed = true;
        }
        changed
    }
}

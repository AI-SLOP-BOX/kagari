use super::*;

pub(super) fn dispatch(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Render(RenderCommands::Render {
            project,
            comp,
            output,
            format,
            frames,
            from,
            to,
            width,
            height,
            exposure,
            lut,
            lut_file,
            color_space,
            step,
            threads,
            skip_existing,
            mfr,
            log_format,
            bindings,
            overrides,
            expressions,
        }) => {
            render::cmd_render(render::RenderArgs {
                project_path: project,
                comp_ref: comp,
                output,
                format,
                frames,
                from,
                to,
                width,
                height,
                exposure,
                lut,
                lut_file,
                color_space,
                step,
                threads,
                skip_existing,
                mfr,
                log_format,
                bindings,
                overrides,
                expressions,
            })?;
        }
        Commands::Render(RenderCommands::Queue {
            project,
            output,
            format,
            threads,
            log_format,
        }) => {
            render::cmd_queue(render::QueueArgs {
                project_path: project,
                output,
                format,
                threads,
                log_format,
            })?;
        }
        Commands::Effects { json } => {
            cmd_effects(json);
        }
        Commands::Info { project } => {
            cmd_info(&project)?;
        }
        Commands::ProductionInfo { project, json } => {
            cmd_production_info(&project, json)?;
        }
        Commands::Bindings {
            project,
            values,
            json,
            apply_frame,
            frame,
            comp,
        } => {
            cmd_bindings(&project, &values, json, apply_frame, frame, comp.as_deref())?;
        }
        Commands::AddSampleBinding {
            project,
            source,
            target,
            points,
            input_min,
            input_max,
            output_min,
            output_max,
        } => {
            cmd_add_sample_binding(
                &project,
                &source,
                &target,
                points,
                (input_min, input_max),
                (output_min, output_max),
            )?;
        }
        Commands::Validate { project } => {
            validate::cmd_validate(&project)?;
        }
        Commands::Doctor { json } => {
            info::cmd_doctor(json)?;
        }
        Commands::Presets { json } => {
            info::cmd_presets(json)?;
        }
        Commands::Render(RenderCommands::Qc {
            project,
            comp,
            frame,
            from,
            to,
            json,
            output,
            fail_on_warnings,
        }) => {
            cmd_qc(QcArgs {
                project_path: project,
                comp_ref: comp,
                frame,
                from,
                to,
                json,
                output,
                fail_on_warnings,
            })?;
        }
        Commands::Script { project, file } => {
            let project_meta = std::fs::metadata(&project).map_err(|e| {
                std::io::Error::new(e.kind(), format!("cannot read project: {}", e))
            })?;
            if project_meta.len() > 50 * 1024 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "project file too large ({} MB, limit 50 MB)",
                        project_meta.len() / (1024 * 1024)
                    ),
                )
                .into());
            }
            let json = kagari_vfx::core::project_migration::read_project_json_file(
                std::path::Path::new(&project),
            )
            .map_err(std::io::Error::other)?;
            let (mut proj, mut production) =
                kagari_vfx::core::project_migration::load_project_document_or_legacy(&json)?;
            // Tolerate wrapped ProjectFile format too
            let source = kagari_vfx::core::project_migration::read_bounded_text_file(
                std::path::Path::new(&file),
                1024 * 1024,
            )
            .map_err(std::io::Error::other)?;
            let logs = kagari_vfx::automation::run_script(&mut proj, &source)?;
            for l in logs {
                println!("{l}");
            }
            if let Some(document) = production.as_mut() {
                *document.project_mut() = proj;
                document
                    .save_atomic(&project)
                    .map_err(std::io::Error::other)?;
            } else {
                kagari_vfx::core::project_migration::save_project_atomic(&proj, &project)?;
            }
            println!("project saved → {}", project);
        }
        Commands::Render(RenderCommands::Frame {
            project,
            frame,
            output,
            width,
            height,
            gpu,
        }) => {
            cmd_frame(&project, frame, &output, width, height, gpu)?;
        }
        Commands::Render(RenderCommands::Lottie { project, output }) => {
            cmd_lottie(&project, &output)?;
        }
        Commands::Render(RenderCommands::Otio {
            project,
            comp,
            output,
            import,
        }) => {
            cmd_otio(&project, comp.as_deref(), &output, import)?;
        }
        Commands::Mocap {
            frames,
            points,
            output,
            block_radius,
            search_radius,
            min_confidence,
            pose,
            fps,
            max_features,
            feature_spacing,
        } => {
            mocap::cmd_mocap(
                &frames,
                &points,
                &output,
                block_radius,
                search_radius,
                min_confidence,
                pose,
                fps,
                max_features,
                feature_spacing,
            )?;
        }
        Commands::Init {
            project,
            name,
            width,
            height,
            fps,
            duration,
        } => {
            cmd_init(&project, &name, width, height, fps, duration)?;
        }
        Commands::AddLayer {
            project,
            name,
            comp,
            kind,
            content,
            in_frame,
            out_frame,
        } => {
            cmd_add_layer(
                &project,
                &name,
                comp.as_deref(),
                &kind,
                content.as_deref(),
                in_frame,
                out_frame,
            )?;
        }
        Commands::Animate {
            project,
            layer,
            property,
            keyframes,
            ease,
            bezier,
            influence,
            comp,
        } => {
            cmd_animate(
                &project,
                &layer,
                comp.as_deref(),
                &property,
                &keyframes,
                &ease,
                bezier.as_deref(),
                influence,
            )?;
        }
        Commands::AddEffect {
            project,
            layer,
            kind,
            param,
            comp,
        } => {
            cmd_add_effect(&project, &layer, comp.as_deref(), &kind, param)?;
        }
        Commands::Trim {
            project,
            layer,
            in_frame,
            out_frame,
            comp,
        } => {
            cmd_trim(&project, &layer, comp.as_deref(), in_frame, out_frame)?;
        }
        Commands::Transition {
            project,
            layer,
            kind,
            duration,
            comp,
        } => {
            cmd_transition(&project, &layer, comp.as_deref(), &kind, duration)?;
        }
        Commands::Parent {
            project,
            layer,
            parent,
            comp,
        } => {
            cmd_parent(&project, &layer, &parent, comp.as_deref())?;
        }
        Commands::Matte {
            project,
            layer,
            mode,
            source,
            comp,
        } => {
            cmd_matte(&project, &layer, &mode, source.as_deref(), comp.as_deref())?;
        }
        Commands::Mask {
            project,
            layer,
            shape,
            coords,
            feather,
            inverted,
            comp,
        } => {
            cmd_mask(
                &project,
                &layer,
                &shape,
                &coords,
                feather,
                inverted,
                comp.as_deref(),
            )?;
        }
        Commands::Beats {
            file,
            fps,
            duration,
            json,
        } => {
            render::cmd_beats(&file, fps, duration, json)?;
        }
        Commands::Preview {
            project,
            comp,
            output,
            scale,
            step,
        } => {
            render::cmd_preview(&project, comp.as_deref(), &output, scale, step)?;
        }
        Commands::Bundle { project, output } => {
            render::cmd_bundle(&project, &output)?;
        }
        Commands::Remap {
            project,
            layer,
            stretch,
            reverse,
            freeze,
            frame_blending,
            comp,
        } => {
            render::cmd_remap(
                &project,
                &layer,
                stretch,
                reverse,
                freeze,
                frame_blending,
                comp.as_deref(),
            )?;
        }
        Commands::Track {
            project,
            layer,
            tracker_idx,
            pos,
            search_size,
            apply_to,
            apply_pos,
            apply_rot,
            comp,
        } => {
            render::cmd_track(
                &project,
                &layer,
                tracker_idx,
                pos.as_deref(),
                search_size,
                apply_to.as_deref(),
                apply_pos,
                apply_rot,
                comp.as_deref(),
            )?;
        }
        Commands::TextAnim {
            project,
            layer,
            preset,
            duration,
            start_frame,
            shape,
            blur,
            tracking,
            comp,
        } => {
            render::cmd_text_anim(
                &project,
                &layer,
                &preset,
                duration,
                start_frame,
                &shape,
                blur,
                tracking,
                comp.as_deref(),
            )?;
        }
        Commands::Camera {
            project,
            pos,
            rot,
            fov,
            focus_dist,
            aperture,
            dof,
            max_blur,
            iris_sides,
            comp,
        } => {
            render::cmd_camera(
                &project,
                pos.as_deref(),
                rot.as_deref(),
                fov,
                focus_dist,
                aperture,
                dof,
                max_blur,
                iris_sides,
                comp.as_deref(),
            )?;
        }
        Commands::ShapeAnim {
            project,
            layer,
            trim_start,
            trim_end,
            trim_duration,
            repeater_copies,
            repeater_offset,
            repeater_rotation,
            repeater_scale,
            comp,
        } => {
            render::cmd_shape_anim(
                &project,
                &layer,
                trim_start,
                trim_end,
                trim_duration,
                repeater_copies,
                repeater_offset.as_deref(),
                repeater_rotation,
                repeater_scale.as_deref(),
                comp.as_deref(),
            )?;
        }
        Commands::Particle {
            project,
            layer,
            preset,
            rate,
            max_particles,
            pos,
            comp,
        } => {
            render::cmd_particle(
                &project,
                &layer,
                &preset,
                rate,
                max_particles,
                pos.as_deref(),
                comp.as_deref(),
            )?;
        }
        Commands::AudioSpectrum {
            project,
            layer,
            audio_file,
            bands,
            min_freq,
            max_freq,
            max_height,
            comp,
        } => {
            render::cmd_audio_spectrum(
                &project,
                &layer,
                &audio_file,
                bands,
                min_freq,
                max_freq,
                max_height,
                comp.as_deref(),
            )?;
        }
        Commands::Roto {
            project,
            layer,
            points,
            kind,
            radius,
            frame,
            comp,
        } => {
            render::cmd_roto(
                &project,
                &layer,
                &points,
                &kind,
                radius,
                frame,
                comp.as_deref(),
            )?;
        }
        Commands::Puppet {
            project,
            layer,
            name,
            pos,
            target,
            frame,
            comp,
        } => {
            render::cmd_puppet(
                &project,
                &layer,
                &name,
                pos.as_deref(),
                target.as_deref(),
                frame,
                comp.as_deref(),
            )?;
        }
        Commands::ChromaKey {
            project,
            layer,
            color,
            gain,
            clip_black,
            clip_white,
            comp,
        } => {
            render::cmd_chroma_key(
                &project,
                &layer,
                &color,
                gain,
                clip_black,
                clip_white,
                comp.as_deref(),
            )?;
        }
        Commands::AudioTrack {
            project,
            layer,
            volume,
            fade_in,
            fade_out,
            comp,
        } => {
            render::cmd_audio_track(&project, &layer, volume, fade_in, fade_out, comp.as_deref())?;
        }
        Commands::Story {
            project,
            title,
            subtitle,
            audio,
            footage,
            width,
            height,
            fps,
            duration_sec,
            theme,
        } => {
            render::cmd_story(
                &project,
                &title,
                &subtitle,
                audio.as_deref(),
                footage.as_deref(),
                width,
                height,
                fps,
                duration_sec,
                &theme,
            )?;
        }
        Commands::Render(RenderCommands::Critique {
            project,
            comp,
            sample_step,
            json,
        }) => {
            render::cmd_critique(&project, comp.as_deref(), sample_step, json)?;
        }
        Commands::Render(RenderCommands::Cinematic {
            project,
            preset,
            grain,
            letterbox,
            aberration,
            vignette,
            comp,
        }) => {
            render::cmd_cinematic(
                &project,
                &preset,
                grain,
                letterbox,
                aberration,
                vignette,
                comp.as_deref(),
            )?;
        }
        Commands::Render(RenderCommands::Mv {
            audio,
            lyrics,
            lyrics_file,
            output,
            style,
            width,
            height,
            fps,
            duration_sec,
            project_only,
        }) => {
            render::cmd_mv(
                &audio,
                lyrics.as_deref(),
                lyrics_file.as_deref(),
                &output,
                &style,
                width,
                height,
                fps,
                duration_sec,
                project_only,
            )?;
        }
    }

    Ok(())
}

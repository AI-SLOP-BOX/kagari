use super::*;

impl GpuComputeContext {
    #[allow(clippy::too_many_arguments)]
    pub fn gpu_color_correct(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        br: f32,
        ct: f32,
        sat: f32,
        hue: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 4,
                angle: 0.0,
                brightness: br,
                contrast: ct,
                saturation: sat,
                hue_shift: hue,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_cc",
        )
    }
    pub fn gpu_sharpen(&self, pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 5,
                angle: 0.0,
                brightness: strength,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_sharp",
        )
    }
    pub fn gpu_threshold(&self, pixels: &mut [u8], w: u32, h: u32, cutoff: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 6,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: cutoff,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_thresh",
        )
    }
    pub fn gpu_emboss(&self, pixels: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 7,
                angle: 0.0,
                brightness: strength,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_emboss",
        )
    }
    pub fn gpu_edge_detect(&self, pixels: &mut [u8], w: u32, h: u32, blend: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 8,
                angle: 0.0,
                brightness: blend,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_edge",
        )
    }
    pub fn gpu_invert(&self, pixels: &mut [u8], w: u32, h: u32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 9,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_inv",
        )
    }
    pub fn gpu_solarize(&self, pixels: &mut [u8], w: u32, h: u32, threshold: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 10,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: threshold,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_solar",
        )
    }
    pub fn gpu_posterize(&self, pixels: &mut [u8], w: u32, h: u32, levels: f32) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 11,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: levels,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_post",
        )
    }

    pub fn gpu_color_tint(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        tint_rgb: [f32; 3],
        intensity: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 12,
                angle: 0.0,
                brightness: tint_rgb[0],
                contrast: tint_rgb[1],
                saturation: tint_rgb[2],
                hue_shift: intensity,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_tint",
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn gpu_drop_shadow(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        color: [f32; 4],
        distance: f32,
        angle: f32,
        blur_r: u32,
    ) -> bool {
        let rad = angle.to_radians();
        let dx = distance * rad.sin();
        let dy = -distance * rad.cos();
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: blur_r.min(32),
                mode: 13,
                angle: 0.0,
                brightness: color[0],
                contrast: color[1],
                saturation: color[2],
                hue_shift: color[3],
                param_f3: dx,
                param_f4: dy,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_shadow",
        )
    }
    pub fn gpu_glow(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        threshold: f32,
        radius: u32,
        intensity: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: radius.min(32),
                mode: 14,
                angle: 0.0,
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: threshold,
                param_f4: intensity,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_glow",
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn gpu_levels(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        in_black: f32,
        in_white: f32,
        gamma: f32,
        out_black: f32,
        out_white: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 15,
                angle: 0.0,
                brightness: in_black / 255.0,
                contrast: in_white / 255.0,
                saturation: gamma,
                hue_shift: out_black / 255.0,
                param_f3: out_white / 255.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_levels",
        )
    }
    pub fn gpu_hue_saturation(
        &self,
        pixels: &mut [u8],
        w: u32,
        h: u32,
        hue_shift: f32,
        saturation: f32,
        lightness: f32,
    ) -> bool {
        self.dispatch_fx(
            pixels,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 16,
                angle: 0.0,
                brightness: hue_shift,
                contrast: saturation,
                saturation: lightness,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_hsl",
        )
    }
    pub fn gpu_color_balance(&self, p: &mut [u8], w: u32, h: u32, shift: [f32; 3]) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 29,
                angle: 0.0,
                brightness: shift[0],
                contrast: shift[1],
                saturation: shift[2],
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_cb",
        )
    }
    pub fn gpu_vibrance(&self, p: &mut [u8], w: u32, h: u32, amount: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 30,
                angle: 0.0,
                brightness: amount,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_vib",
        )
    }
    pub fn gpu_white_balance(&self, p: &mut [u8], w: u32, h: u32, temp: f32, tint: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 31,
                angle: 0.0,
                brightness: temp,
                contrast: tint,
                saturation: 1.0,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_wb",
        )
    }
    pub fn gpu_hsl_adjust(
        &self,
        p: &mut [u8],
        w: u32,
        h: u32,
        hue: f32,
        sat: f32,
        light: f32,
    ) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 32,
                angle: 0.0,
                brightness: hue,
                contrast: sat,
                saturation: light,
                hue_shift: 0.0,
                param_f3: 0.0,
                param_f4: 0.0,
                param_f5: 0.0,
                param_f6: 0.0,
                param_f7: 0.0,
                param_f8: 0.0,
                _pad: 0.0,
            },
            "gpu_hsl",
        )
    }
}

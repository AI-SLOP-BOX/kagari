use super::*;

impl GpuComputeContext {
    pub fn gpu_cc_lens(&self, p: &mut [u8], w: u32, h: u32, convergence: f32, zoom: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 40,
                angle: 0.0,
                brightness: convergence,
                contrast: zoom,
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
            "gpu_lens",
        )
    }
    pub fn gpu_polar_coords(&self, p: &mut [u8], w: u32, h: u32, to_polar: bool) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 41,
                angle: 0.0,
                brightness: if to_polar { 1.0 } else { 0.0 },
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
            "gpu_polar",
        )
    }
    pub fn gpu_optics_comp(&self, p: &mut [u8], w: u32, h: u32, fov: f32, reverse: bool) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 42,
                angle: 0.0,
                brightness: fov,
                contrast: if reverse { 1.0 } else { 0.0 },
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
            "gpu_oc",
        )
    }
    pub fn gpu_fisheye(&self, p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 43,
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
            "gpu_fish",
        )
    }
    pub fn gpu_lens_correction(&self, p: &mut [u8], w: u32, h: u32, k1: f32, k2: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 44,
                angle: 0.0,
                brightness: k1,
                contrast: k2,
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
            "gpu_lc",
        )
    }
    pub fn gpu_vortex(&self, p: &mut [u8], w: u32, h: u32, angle: f32, radius: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 45,
                angle: 0.0,
                brightness: angle,
                contrast: radius,
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
            "gpu_vtx",
        )
    }
    pub fn gpu_pinch_punch(&self, p: &mut [u8], w: u32, h: u32, strength: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 46,
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
            "gpu_pp",
        )
    }
    pub fn gpu_refraction(&self, p: &mut [u8], w: u32, h: u32, ior: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 47,
                angle: 0.0,
                brightness: ior,
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
            "gpu_refr",
        )
    }
    pub fn gpu_bend_it(&self, p: &mut [u8], w: u32, h: u32, top: f32, bottom: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 48,
                angle: 0.0,
                brightness: top,
                contrast: bottom,
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
            "gpu_bend",
        )
    }
    pub fn gpu_tiler(&self, p: &mut [u8], w: u32, h: u32, scale: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 49,
                angle: 0.0,
                brightness: scale,
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
            "gpu_tile",
        )
    }
}

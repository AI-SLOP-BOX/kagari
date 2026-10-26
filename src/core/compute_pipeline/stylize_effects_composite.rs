use super::*;

impl GpuComputeContext {
    pub fn gpu_reflection_map(
        &self,
        p: &mut [u8],
        w: u32,
        h: u32,
        reflect_y: bool,
        fade: f32,
    ) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 62,
                angle: 0.0,
                brightness: if reflect_y { 1.0 } else { 0.0 },
                contrast: fade,
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
            "gpu_rm",
        )
    }
    pub fn gpu_venetian_blinds(
        &self,
        p: &mut [u8],
        w: u32,
        h: u32,
        completion: f32,
        width: f32,
    ) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 36,
                angle: 0.0,
                brightness: completion,
                contrast: width,
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
            "gpu_vb",
        )
    }
    pub fn gpu_letterbox(&self, p: &mut [u8], w: u32, h: u32, frac: f32) -> bool {
        self.dispatch_fx(
            p,
            w,
            h,
            ParamsUniform {
                width: w,
                height: h,
                radius: 0,
                mode: 39,
                angle: 0.0,
                brightness: frac,
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
            "gpu_lb",
        )
    }
}

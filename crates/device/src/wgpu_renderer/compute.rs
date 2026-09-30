//! Executing a compute pass.
//!
//! A compute pass runs even though it contributes no pixels, because it may be a
//! prerequisite for the passes that follow. It binds the same way a rasterize pass does,
//! so one shader author writes one binding convention rather than two.

use definition::Value;
use graphics::{PassView, resolve_for_pass};

use super::WgpuRenderer;
use super::bindings::TextureCache;
use crate::render::RenderError;

impl WgpuRenderer {
    /// Executes one compute pass.
    pub(super) fn run_compute(
        &self,
        pass: &PassView<'_>,
        instance: &Value,
        cache: &mut TextureCache,
    ) -> Result<(), RenderError> {
        let label = pass.label().unwrap_or("<unlabelled>").to_owned();
        let shader = resolve_for_pass(&label, instance, pass.shader_binder()?)
            .map_err(RenderError::Graphics)?;
        let module = self.shader_module(&shader)?;
        if let Some(schema) = graphics::uniform_schema(pass, instance)? {
            self.check_uniform_layout(&shader, &schema)?;
        }
        let bindings = self.bind_pass(pass, instance, cache)?;

        let workgroup: Vec<u32> = pass
            .workgroup()?
            .iter()
            .filter_map(Value::as_int)
            .map(|value| value.max(0) as u32)
            .collect();
        if workgroup.len() < 6 {
            return Err(RenderError::Backend(
                "a compute pass needs a six component workgroup".to_owned(),
            ));
        }
        // The workgroup size is the shader's own declaration; the dispatch count is how
        // many groups of it to run, which is the second triple of the field.
        let count = [
            workgroup.get(3).copied().unwrap_or(1).max(1),
            workgroup.get(4).copied().unwrap_or(1).max(1),
            workgroup.get(5).copied().unwrap_or(1).max(1),
        ];

        let bind_group_layout = self.compute_bind_group_layout(&bindings);
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("whiting compute layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("whiting compute"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some(pass.entry_point().unwrap_or("main")),
                compilation_options: Default::default(),
                cache: None,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("whiting compute bind group"),
            layout: &bind_group_layout,
            entries: &bindings.group_entries(),
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("whiting compute encoder"),
            });
        {
            let mut pass_encoder = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(&label),
                timestamp_writes: None,
            });
            pass_encoder.set_pipeline(&pipeline);
            pass_encoder.set_bind_group(0, &bind_group, &[]);
            pass_encoder.dispatch_workgroups(count[0], count[1], count[2]);
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Builds the bind group layout a compute pass needs.
    fn compute_bind_group_layout(
        &self,
        bindings: &super::bindings::Bindings,
    ) -> wgpu::BindGroupLayout {
        self.device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("whiting compute bind group layout"),
                entries: &bindings.layout_entries(wgpu::ShaderStages::COMPUTE),
            })
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! WGSL compute backend for field diffusion.
//!
//! The CPU stepper remains the reference implementation. This backend exists to
//! prove the shader path against the same `FieldStepper` contract.

use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::{FieldGrid, FieldLayer, FieldStepError, FieldStepRequest, FieldStepper};

const SHADER: &str = r#"
struct Params {
    width: u32,
    height: u32,
    diffusion: f32,
    decay: f32,
    dt: f32,
    max_value: f32,
    _pad0: u32,
    _pad1: u32,
};

@group(0) @binding(0) var<storage, read> input_field: array<f32>;
@group(0) @binding(1) var<storage, read> obstacle_field: array<f32>;
@group(0) @binding(2) var<storage, read> source_field: array<f32>;
@group(0) @binding(3) var<storage, read_write> output_field: array<f32>;
@group(0) @binding(4) var<uniform> params: Params;

fn idx(x: u32, y: u32) -> u32 {
    return y * params.width + x;
}

fn sanitize_concentration(value: f32) -> f32 {
    if !isFinite(value) {
        return 0.0;
    }
    return clamp(value, 0.0, params.max_value);
}

fn sanitize_source(value: f32) -> f32 {
    if !isFinite(value) {
        return 0.0;
    }
    return max(value, 0.0);
}

fn neighbor(x: i32, y: i32, fallback: f32) -> f32 {
    if x < 0 || y < 0 || x >= i32(params.width) || y >= i32(params.height) {
        return fallback;
    }
    let offset = idx(u32(x), u32(y));
    if obstacle_field[offset] >= 0.5 {
        return fallback;
    }
    return input_field[offset];
}

@compute @workgroup_size(8, 8, 1)
fn step(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let offset = idx(id.x, id.y);
    if obstacle_field[offset] >= 0.5 {
        output_field[offset] = 0.0;
        return;
    }

    let center = input_field[offset];
    let x = i32(id.x);
    let y = i32(id.y);
    let left = neighbor(x - 1, y, center);
    let right = neighbor(x + 1, y, center);
    let up = neighbor(x, y - 1, center);
    let down = neighbor(x, y + 1, center);
    let laplacian = left + right + up + down - 4.0 * center;
    let source = sanitize_source(source_field[offset]);
    let next = center + params.dt * (params.diffusion * laplacian - params.decay * center + source);
    output_field[offset] = sanitize_concentration(next);
}
"#;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuParams {
    width: u32,
    height: u32,
    diffusion: f32,
    decay: f32,
    dt: f32,
    max_value: f32,
    _pad0: u32,
    _pad1: u32,
}

/// Single-dispatch WGSL implementation of `FieldStepper`.
///
/// This is intentionally simple: create buffers, dispatch one compute pass,
/// read back the selected layer. Long-lived buffer reuse can be added once the
/// parity contract is stable.
#[derive(Debug, Clone, Copy, Default)]
pub struct WgslFieldStepper;

impl FieldStepper for WgslFieldStepper {
    fn step(
        &self,
        field: &mut FieldGrid,
        request: &FieldStepRequest,
    ) -> Result<(), FieldStepError> {
        pollster::block_on(step_async(field, request))
    }
}

async fn step_async(
    field: &mut FieldGrid,
    request: &FieldStepRequest,
) -> Result<(), FieldStepError> {
    step_async_with_backends(field, request, wgpu::Backends::all()).await
}

async fn step_async_with_backends(
    field: &mut FieldGrid,
    request: &FieldStepRequest,
    backends: wgpu::Backends,
) -> Result<(), FieldStepError> {
    request.params.validate()?;
    let cell_count = field.width() * field.height();
    if request.source.len() != cell_count {
        return Err(FieldStepError::GpuDispatchFailed(format!(
            "source length {} does not match field cell count {cell_count}",
            request.source.len()
        )));
    }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .map_err(|e| FieldStepError::GpuUnavailable(format!("adapter request failed: {e}")))?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("symtropy-lifesim-core"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| FieldStepError::GpuUnavailable(format!("device request failed: {e}")))?;

    let input = &field.channels[request.layer.index()];
    let obstacle = &field.channels[FieldLayer::Obstacle.index()];
    // Sanitize host-provided source values before upload rather than relying on
    // shader-side checks for NaN/Infinity under WGSL's finite-math assumptions.
    let sanitized_source: Vec<f32> = request
        .source
        .iter()
        .map(|&value| if value.is_finite() { value.max(0.0) } else { 0.0 })
        .collect();
    let output = vec![0.0f32; cell_count];
    let params = GpuParams {
        width: field.width() as u32,
        height: field.height() as u32,
        diffusion: request.params.diffusion,
        decay: request.params.decay,
        dt: request.params.dt,
        max_value: request.params.max_value,
        _pad0: 0,
        _pad1: 0,
    };

    let input_buffer = storage_buffer(&device, "field-input", input, wgpu::BufferUsages::STORAGE);
    let obstacle_buffer = storage_buffer(
        &device,
        "field-obstacle",
        obstacle,
        wgpu::BufferUsages::STORAGE,
    );
    let source_buffer = storage_buffer(
        &device,
        "field-source",
        &sanitized_source,
        wgpu::BufferUsages::STORAGE,
    );
    let output_buffer = storage_buffer(
        &device,
        "field-output",
        &output,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    );
    let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("field-params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("field-readback"),
        size: std::mem::size_of_val(output.as_slice()) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("field-diffuse-decay"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("field-bind-group-layout"),
        entries: &[
            storage_entry(0, true),
            storage_entry(1, true),
            storage_entry(2, true),
            storage_entry(3, false),
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("field-pipeline-layout"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("field-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("step"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("field-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            bind_entry(0, &input_buffer),
            bind_entry(1, &obstacle_buffer),
            bind_entry(2, &source_buffer),
            bind_entry(3, &output_buffer),
            bind_entry(4, &params_buffer),
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("field-command-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("field-compute-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(
            (field.width() as u32).div_ceil(8),
            (field.height() as u32).div_ceil(8),
            1,
        );
    }
    encoder.copy_buffer_to_buffer(
        &output_buffer,
        0,
        &readback_buffer,
        0,
        std::mem::size_of_val(output.as_slice()) as u64,
    );
    queue.submit(Some(encoder.finish()));

    let slice = readback_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| FieldStepError::GpuDispatchFailed(e.to_string()))?;
    rx.recv()
        .map_err(|e| FieldStepError::GpuDispatchFailed(e.to_string()))?
        .map_err(|e| FieldStepError::GpuDispatchFailed(e.to_string()))?;

    let mapped = slice.get_mapped_range();
    let values: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
    drop(mapped);
    readback_buffer.unmap();

    field.channels[request.layer.index()] = values;
    Ok(())
}

fn storage_buffer(
    device: &wgpu::Device,
    label: &'static str,
    values: &[f32],
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: bytemuck::cast_slice(values),
        usage,
    })
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bind_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}


/// An explicitly owned WGPU runtime for repeated field diffusion steps.
///
/// The instance, adapter, device, shader, and compute pipeline live for the
/// lifetime of this value. Buffers and the bind group are reused while field
/// dimensions remain unchanged and replaced atomically on dimension changes.
/// No global cache is used; mutable access serializes submissions through this
/// runtime and prevents unsynchronized buffer reuse by concurrent callers.
pub struct PersistentWgslFieldRuntime {
    // Keep the WGPU instance alive for the full runtime lifetime.
    _instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    adapter_info: wgpu::AdapterInfo,
    buffers: Option<PersistentFieldBuffers>,
    // Any failure after submit leaves completion/mapping state uncertain.
    // Do not reuse this runtime; construct a fresh runtime instead.
    poisoned: Option<String>,
}

struct PersistentFieldBuffers {
    width: u32,
    height: u32,
    cell_count: usize,
    input: wgpu::Buffer,
    obstacle: wgpu::Buffer,
    source: wgpu::Buffer,
    source_staging: Vec<f32>,
    output: wgpu::Buffer,
    params: wgpu::Buffer,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl PersistentWgslFieldRuntime {
    /// Construct a persistent runtime using any configured WGPU backend.
    pub fn new() -> Result<Self, FieldStepError> {
        Self::new_with_backends(wgpu::Backends::all())
    }

    fn new_with_backends(backends: wgpu::Backends) -> Result<Self, FieldStepError> {
        pollster::block_on(Self::new_async(backends))
    }

    async fn new_async(backends: wgpu::Backends) -> Result<Self, FieldStepError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| FieldStepError::GpuUnavailable(format!("adapter request failed: {error}")))?;
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("symtropy-persistent-lifesim"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| FieldStepError::GpuUnavailable(format!("device request failed: {error}")))?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("field-diffuse-decay-persistent"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("field-persistent-bind-group-layout"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("field-persistent-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("field-persistent-pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("step"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Ok(Self {
            _instance: instance,
            device,
            queue,
            pipeline,
            bind_group_layout,
            adapter_info,
            buffers: None,
            poisoned: None,
        })
    }

    /// Return the actual adapter selected by WGPU; backend identity is not inferred.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Execute one field step, reusing device resources until dimensions change.
    ///
    /// The input field is mutated only after submission completes and readback
    /// succeeds. Invalid dimensions, resource limits, device failures, or map
    /// failures return an error without publishing partial field output.
    pub fn step(
        &mut self,
        field: &mut FieldGrid,
        request: &FieldStepRequest,
    ) -> Result<(), FieldStepError> {
        if let Some(reason) = &self.poisoned {
            return Err(FieldStepError::GpuDispatchFailed(format!(
                "persistent WGPU runtime is poisoned after an uncertain execution state: {reason}"
            )));
        }
        request.params.validate()?;
        let width = u32::try_from(field.width()).map_err(|_| {
            FieldStepError::GpuDispatchFailed("field width exceeds the WGSL u32 index space".into())
        })?;
        let height = u32::try_from(field.height()).map_err(|_| {
            FieldStepError::GpuDispatchFailed("field height exceeds the WGSL u32 index space".into())
        })?;
        if width == 0 || height == 0 {
            return Err(FieldStepError::GpuDispatchFailed(
                "field dimensions must both be non-zero".into(),
            ));
        }
        let cell_count = field.width().checked_mul(field.height()).ok_or_else(|| {
            FieldStepError::GpuDispatchFailed("field cell count overflow".into())
        })?;
        if request.source.len() != cell_count {
            return Err(FieldStepError::GpuDispatchFailed(format!(
                "source length {} does not match field cell count {cell_count}",
                request.source.len()
            )));
        }
        let byte_size = u64::try_from(cell_count)
            .ok()
            .and_then(|count| count.checked_mul(std::mem::size_of::<f32>() as u64))
            .ok_or_else(|| FieldStepError::GpuDispatchFailed("field buffer byte size overflow".into()))?;
        let workgroups_x = width.div_ceil(8);
        let workgroups_y = height.div_ceil(8);
        let limits = self.device.limits();
        if byte_size > limits.max_buffer_size {
            return Err(FieldStepError::GpuDispatchFailed(format!(
                "field buffer requires {byte_size} bytes, device max_buffer_size is {}",
                limits.max_buffer_size
            )));
        }
        if byte_size > u64::from(limits.max_storage_buffer_binding_size) {
            return Err(FieldStepError::GpuDispatchFailed(format!(
                "field storage binding requires {byte_size} bytes, device max_storage_buffer_binding_size is {}",
                limits.max_storage_buffer_binding_size
            )));
        }
        if workgroups_x > limits.max_compute_workgroups_per_dimension
            || workgroups_y > limits.max_compute_workgroups_per_dimension
        {
            return Err(FieldStepError::GpuDispatchFailed(format!(
                "field dispatch ({workgroups_x}, {workgroups_y}, 1) exceeds device per-dimension workgroup limit {}",
                limits.max_compute_workgroups_per_dimension
            )));
        }

        self.ensure_buffers(width, height, cell_count, byte_size)?;
        let input = &field.channels[request.layer.index()];
        let obstacle = &field.channels[FieldLayer::Obstacle.index()];
        if input.len() != cell_count || obstacle.len() != cell_count {
            return Err(FieldStepError::GpuDispatchFailed(
                "field channels do not match checked field dimensions".into(),
            ));
        }
        let params = GpuParams {
            width,
            height,
            diffusion: request.params.diffusion,
            decay: request.params.decay,
            dt: request.params.dt,
            max_value: request.params.max_value,
            _pad0: 0,
            _pad1: 0,
        };
        let buffers = self.buffers.as_mut().ok_or_else(|| {
            FieldStepError::GpuDispatchFailed("persistent field buffers were not initialized".into())
        })?;
        for (packed, &value) in buffers.source_staging.iter_mut().zip(&request.source) {
            *packed = if value.is_finite() { value.max(0.0) } else { 0.0 };
        }
        self.queue.write_buffer(&buffers.input, 0, bytemuck::cast_slice(input));
        self.queue.write_buffer(&buffers.obstacle, 0, bytemuck::cast_slice(obstacle));
        self.queue.write_buffer(&buffers.source, 0, bytemuck::cast_slice(&buffers.source_staging));
        self.queue.write_buffer(&buffers.params, 0, bytemuck::bytes_of(&params));

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("field-persistent-command-encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("field-persistent-compute-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        encoder.copy_buffer_to_buffer(&buffers.output, 0, &buffers.readback, 0, byte_size);
        self.queue.submit(Some(encoder.finish()));

        let slice = buffers.readback.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        if let Err(error) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
            let reason = format!("device poll failed after submission: {error}");
            self.poisoned = Some(reason.clone());
            return Err(FieldStepError::GpuDispatchFailed(reason));
        }
        let map_result = match rx.recv() {
            Ok(result) => result,
            Err(error) => {
                let reason = format!("readback callback failed after submission: {error}");
                self.poisoned = Some(reason.clone());
                return Err(FieldStepError::GpuDispatchFailed(reason));
            }
        };
        if let Err(error) = map_result {
            let reason = format!("readback mapping failed after submission: {error}");
            self.poisoned = Some(reason.clone());
            return Err(FieldStepError::GpuDispatchFailed(reason));
        }
        let mapped = slice.get_mapped_range();
        let values: Vec<f32> = bytemuck::cast_slice(&mapped).to_vec();
        drop(mapped);
        buffers.readback.unmap();
        if values.len() != cell_count {
            let reason = format!(
                "readback returned {} cells, expected {cell_count}",
                values.len()
            );
            self.poisoned = Some(reason.clone());
            return Err(FieldStepError::GpuDispatchFailed(reason));
        }
        field.channels[request.layer.index()] = values;
        Ok(())
    }

    fn ensure_buffers(
        &mut self,
        width: u32,
        height: u32,
        cell_count: usize,
        byte_size: u64,
    ) -> Result<(), FieldStepError> {
        if self.buffers.as_ref().is_some_and(|buffers| {
            buffers.width == width && buffers.height == height && buffers.cell_count == cell_count
        }) {
            return Ok(());
        }

        let input = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-input"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let obstacle = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-obstacle"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let source = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-source"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let output = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-output"),
            size: byte_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-params"),
            size: std::mem::size_of::<GpuParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("field-persistent-readback"),
            size: byte_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("field-persistent-bind-group"),
            layout: &self.bind_group_layout,
            entries: &[
                bind_entry(0, &input),
                bind_entry(1, &obstacle),
                bind_entry(2, &source),
                bind_entry(3, &output),
                bind_entry(4, &params),
            ],
        });
        self.buffers = Some(PersistentFieldBuffers {
            width,
            height,
            cell_count,
            input,
            obstacle,
            source,
            source_staging: vec![0.0; cell_count],
            output,
            params,
            readback,
            bind_group,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CpuFieldStepper, DiffusionParams, FieldLayer, FieldStepRequest,
        compare_layer_within_epsilon,
    };

    #[test]
    fn persistent_runtime_matches_cpu_reference_over_repeated_steps_and_resize() {
        let require_adapter = std::env::var_os("SYMTROPY_REQUIRE_WGPU_ADAPTER").is_some();
        let require_vulkan = std::env::var_os("SYMTROPY_REQUIRE_WGPU_VULKAN").is_some();
        let initialization_started = std::time::Instant::now();
        let runtime_result = if require_vulkan {
            PersistentWgslFieldRuntime::new_with_backends(wgpu::Backends::VULKAN)
        } else {
            PersistentWgslFieldRuntime::new()
        };
        let initialization_elapsed = initialization_started.elapsed();
        let mut runtime = match runtime_result {
            Ok(runtime) => runtime,
            Err(FieldStepError::GpuUnavailable(message))
                if !require_adapter && message.starts_with("adapter request failed:") =>
            {
                eprintln!("skipping persistent WGSL parity test: {message}");
                return;
            }
            Err(error) => panic!("persistent WGPU runtime initialization failed: {error}"),
        };
        if require_vulkan {
            assert_eq!(
                runtime.adapter_info().backend,
                wgpu::Backend::Vulkan,
                "qualification must execute through WGPU's Vulkan backend"
            );
        }
        println!("selected_adapter={:?}", runtime.adapter_info());
        println!(
            "cold_runtime_initialization_ms={:.3}",
            initialization_elapsed.as_secs_f64() * 1_000.0
        );

        // Reuse the same runtime for multiple steps, then reuse it again after
        // a shape change. Source sanitization must agree with the CPU oracle,
        // including negative and non-finite inputs.
        for (width, height) in [(8, 6), (13, 7)] {
            let mut cpu = FieldGrid::new(width, height);
            let mut gpu = FieldGrid::new(width, height);
            for field in [&mut cpu, &mut gpu] {
                field.set(FieldLayer::Nutrient, width / 2, height / 2, 15.0);
                field.set(FieldLayer::Nutrient, width - 1, 0, 2.0);
                field.set(FieldLayer::Obstacle, width / 2 + 1, height / 2, 1.0);
            }
            let mut source = vec![0.0; width * height];
            source[0] = f32::NAN;
            source[1] = f32::INFINITY;
            source[2] = f32::NEG_INFINITY;
            source[3] = -5.0;
            source[width * height - 1] = 4.0;
            let request = FieldStepRequest {
                layer: FieldLayer::Nutrient,
                source,
                params: DiffusionParams {
                    diffusion: 0.08,
                    decay: 0.02,
                    dt: 1.0,
                    max_value: 1_000.0,
                },
            };

            for step_index in 0..5 {
                let cpu_started = std::time::Instant::now();
                CpuFieldStepper
                    .step(&mut cpu, &request)
                    .expect("CPU reference step must succeed");
                let cpu_elapsed = cpu_started.elapsed();
                let gpu_started = std::time::Instant::now();
                runtime
                    .step(&mut gpu, &request)
                    .unwrap_or_else(|error| panic!("persistent GPU step {step_index} failed: {error}"));
                let gpu_elapsed = gpu_started.elapsed();
                println!(
                    "field_step_timing grid={}x{} step={} cpu_ms={:.3} gpu_end_to_end_upload_submit_completion_readback_ms={:.3}",
                    width,
                    height,
                    step_index,
                    cpu_elapsed.as_secs_f64() * 1_000.0,
                    gpu_elapsed.as_secs_f64() * 1_000.0
                );
                let report = compare_layer_within_epsilon(
                    &cpu,
                    &gpu,
                    FieldLayer::Nutrient,
                    0.0001,
                );
                assert!(
                    report.within_epsilon(),
                    "step {step_index}, grid {width}x{height}, report={report:?}"
                );
            }
        }
    }

    #[test]
    fn wgsl_stepper_matches_cpu_reference_with_source_and_obstacles() {
        let mut cpu = FieldGrid::new(8, 6);
        let mut gpu = FieldGrid::new(8, 6);
        for field in [&mut cpu, &mut gpu] {
            field.set(FieldLayer::Nutrient, 3, 3, 15.0);
            field.set(FieldLayer::Nutrient, 5, 1, 2.0);
            field.set(FieldLayer::Obstacle, 4, 3, 1.0);
        }
        let mut source = vec![0.0; cpu.width() * cpu.height()];
        source[cpu.idx(2, 2)] = 4.0;
        // CPU and GPU must agree on malformed source values, not just ordinary
        // finite values. The GPU path sanitizes these before storage upload.
        source[0] = f32::NAN;
        source[1] = f32::INFINITY;
        source[2] = f32::NEG_INFINITY;
        source[3] = -5.0;
        let request = FieldStepRequest {
            layer: FieldLayer::Nutrient,
            source,
            params: DiffusionParams {
                diffusion: 0.08,
                decay: 0.02,
                dt: 1.0,
                max_value: 1_000.0,
            },
        };

        CpuFieldStepper.step(&mut cpu, &request).unwrap();
        let require_adapter = std::env::var_os("SYMTROPY_REQUIRE_WGPU_ADAPTER").is_some();
        let require_vulkan = std::env::var_os("SYMTROPY_REQUIRE_WGPU_VULKAN").is_some();
        let step_result = if require_vulkan {
            pollster::block_on(step_async_with_backends(
                &mut gpu,
                &request,
                wgpu::Backends::VULKAN,
            ))
        } else {
            WgslFieldStepper.step(&mut gpu, &request)
        };
        match step_result {
            Ok(()) => {}
            Err(FieldStepError::GpuUnavailable(message))
                if !require_adapter && message.starts_with("adapter request failed:") =>
            {
                eprintln!("skipping WGSL parity test: {message}");
                return;
            }
            Err(error) => panic!("WGSL field step failed: {error}"),
        }

        let report = compare_layer_within_epsilon(&cpu, &gpu, FieldLayer::Nutrient, 0.0001);
        assert!(report.within_epsilon(), "report={report:?}");
    }
}

use std::fs;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::render_asset::{RenderAssetUsages, RenderAssets as GpuRenderAssets};
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, ImageCopyBuffer,
    ImageDataLayout, Maintain, MapMode, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::{ImageSampler, TextureFormatPixelInfo};
use bevy::render::{Render, RenderApp, RenderSet};
use crossbeam_channel::{Receiver, Sender};

use crate::autobattler::constants::COPY_BYTES_PER_ROW_ALIGNMENT;
use crate::autobattler::state::{AppScreen, AutobattlerState, SpriteReviewStage, SpriteReviewState};

#[derive(Resource)]
pub struct ScreenshotState {
    pub requested: bool,
    pub requested_path: Option<String>,
    pub next_index: u32,
    pub last_path: Option<String>,
    pub last_error: Option<String>,
    pub capture_count: u32,
    pub max_auto_captures: Option<u32>,
    pub headless_enabled: bool,
    pub auto_allowed: bool,
    pub auto_enabled: bool,
    pub interval_seconds: f32,
    pub elapsed_seconds: f32,
    pub use_latest_path: bool,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        Self {
            requested: false,
            requested_path: None,
            next_index: 0,
            last_path: None,
            last_error: None,
            capture_count: 0,
            max_auto_captures: None,
            headless_enabled: false,
            auto_allowed: false,
            auto_enabled: false,
            interval_seconds: 1.0,
            elapsed_seconds: 0.0,
            use_latest_path: true,
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct HeadlessConfig {
    pub size: UVec2,
    pub format: TextureFormat,
}

#[derive(Resource, Clone)]
pub struct HeadlessRenderTarget {
    pub image: Handle<Image>,
    pub size: UVec2,
    pub format: TextureFormat,
}

impl ExtractResource for HeadlessRenderTarget {
    type Source = HeadlessRenderTarget;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}

#[derive(Resource, Clone)]
pub struct HeadlessScreenshotChannels {
    request_tx: Sender<HeadlessScreenshotRequest>,
    request_rx: Receiver<HeadlessScreenshotRequest>,
    result_tx: Sender<HeadlessScreenshotResult>,
    result_rx: Receiver<HeadlessScreenshotResult>,
}

#[derive(Resource, Clone)]
struct HeadlessScreenshotRenderChannels {
    request_rx: Receiver<HeadlessScreenshotRequest>,
    result_tx: Sender<HeadlessScreenshotResult>,
}

#[derive(Clone)]
struct HeadlessScreenshotRequest {
    path: String,
}

#[derive(Clone)]
struct HeadlessScreenshotResult {
    path: String,
    error: Option<String>,
}

pub struct HeadlessScreenshotPlugin;

impl Plugin for HeadlessScreenshotPlugin {
    fn build(&self, app: &mut App) {
        let (request_tx, request_rx) = crossbeam_channel::unbounded();
        let (result_tx, result_rx) = crossbeam_channel::unbounded();
        app.insert_resource(HeadlessScreenshotChannels {
            request_tx,
            request_rx,
            result_tx,
            result_rx,
        });
        app.add_plugins(ExtractResourcePlugin::<HeadlessRenderTarget>::default());
        app.add_systems(Update, headless_results_system);
    }

    fn finish(&self, app: &mut App) {
        let channels = app.world.resource::<HeadlessScreenshotChannels>().clone();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(HeadlessScreenshotRenderChannels {
                request_rx: channels.request_rx.clone(),
                result_tx: channels.result_tx.clone(),
            });
            render_app.add_systems(
                Render,
                headless_capture_render_system.in_set(RenderSet::Cleanup),
            );
        }
    }
}

pub fn screenshot_system(
    time: Res<Time>,
    state: Res<AutobattlerState>,
    mut screenshots: ResMut<ScreenshotState>,
    headless_target: Option<Res<HeadlessRenderTarget>>,
    channels: Option<Res<HeadlessScreenshotChannels>>,
) {
    let headless = headless_target.is_some();
    if !screenshots.auto_allowed {
        screenshots.auto_enabled = false;
    }
    if screenshots.auto_allowed
        && screenshots.auto_enabled
        && matches!(state.app.screen, AppScreen::Run)
    {
        let interval = screenshots.interval_seconds.max(0.1);
        screenshots.elapsed_seconds += time.delta_seconds();
        if screenshots.elapsed_seconds >= interval {
            screenshots.elapsed_seconds = 0.0;
            screenshots.requested = true;
        }
    } else {
        screenshots.elapsed_seconds = 0.0;
    }

    if !screenshots.requested {
        return;
    }
    screenshots.requested = false;
    screenshots.last_error = None;
    if let Err(err) = fs::create_dir_all("screenshots") {
        screenshots.last_error = Some(format!("Screenshot folder error: {err}"));
        return;
    }
    let path = if let Some(custom) = screenshots.requested_path.take() {
        custom
    } else if screenshots.use_latest_path {
        "screenshots/latest.png".to_string()
    } else {
        let path = format!("screenshots/autobattler_{:04}.png", screenshots.next_index);
        screenshots.next_index = screenshots.next_index.wrapping_add(1);
        path
    };
    if !headless {
        screenshots.last_error =
            Some("Headless screenshots require --headless-screenshots.".to_string());
        return;
    }
    let Some(channels) = channels else {
        screenshots.last_error = Some("Headless screenshot channels missing.".to_string());
        return;
    };
    if let Err(err) = channels
        .request_tx
        .send(HeadlessScreenshotRequest { path: path.clone() })
    {
        screenshots.last_error = Some(format!("Headless screenshot request failed: {err}"));
        return;
    }
}

fn headless_results_system(
    mut screenshots: ResMut<ScreenshotState>,
    channels: Res<HeadlessScreenshotChannels>,
    mut app_exit: EventWriter<AppExit>,
    state: Res<AutobattlerState>,
    mut review: Option<ResMut<SpriteReviewState>>,
) {
    for result in channels.result_rx.try_iter() {
        if let Some(err) = result.error {
            screenshots.last_error = Some(err);
            continue;
        }
        screenshots.last_error = None;
        screenshots.last_path = Some(result.path);
        screenshots.capture_count = screenshots.capture_count.saturating_add(1);
        if let Some(max) = screenshots.max_auto_captures {
            if screenshots.capture_count >= max {
                app_exit.send(AppExit);
            }
        }
        if let Some(review) = review.as_mut() {
            if matches!(state.app.screen, AppScreen::SpriteReview) && review.awaiting_capture {
                review.awaiting_capture = false;
                review.frames_since_refresh = 0;
                match review.stage {
                    SpriteReviewStage::Weapons => {
                        if review.race_index + 1 < review.races.len() {
                            review.race_index += 1;
                            review.needs_refresh = true;
                        } else {
                            review.stage = SpriteReviewStage::Pained;
                            review.needs_refresh = true;
                        }
                    }
                    SpriteReviewStage::Pained => {
                        app_exit.send(AppExit);
                    }
                }
            }
        }
    }
}

fn aligned_bytes_per_row(bytes_per_row: u32) -> u32 {
    let align = COPY_BYTES_PER_ROW_ALIGNMENT;
    if bytes_per_row % align == 0 {
        bytes_per_row
    } else {
        bytes_per_row + (align - (bytes_per_row % align))
    }
}

fn readback_buffer(
    render_device: &RenderDevice,
    buffer: Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
    pixel_size: u32,
) -> Result<Vec<u8>, String> {
    let buffer_slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    render_device.poll(Maintain::Wait);
    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(format!("Screenshot map error: {err}")),
        Err(err) => return Err(format!("Screenshot map channel error: {err}")),
    }
    let mut data = buffer_slice.get_mapped_range().to_vec();
    drop(buffer_slice);
    buffer.unmap();

    let unpadded_bytes_per_row = (width * pixel_size) as usize;
    let padded_bytes_per_row = padded_bytes_per_row as usize;
    if padded_bytes_per_row != unpadded_bytes_per_row {
        let mut trimmed = Vec::with_capacity(unpadded_bytes_per_row * height as usize);
        for row in data.chunks_exact(padded_bytes_per_row) {
            trimmed.extend_from_slice(&row[..unpadded_bytes_per_row]);
        }
        data = trimmed;
    }
    Ok(data)
}

fn save_headless_image(
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: TextureFormat,
    path: &str,
) -> Result<(), String> {
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::RENDER_WORLD,
    );
    let dyn_image = image
        .try_into_dynamic()
        .map_err(|err| format!("Screenshot format error: {err}"))?;
    dyn_image
        .to_rgb8()
        .save(path)
        .map_err(|err| format!("Screenshot save error: {err}"))?;
    Ok(())
}

fn headless_capture_render_system(
    target: Option<Res<HeadlessRenderTarget>>,
    channels: Res<HeadlessScreenshotRenderChannels>,
    gpu_images: Res<GpuRenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let Some(target) = target else {
        return;
    };
    let Some(gpu_image) = gpu_images.get(&target.image) else {
        return;
    };
    let width = target.size.x;
    let height = target.size.y;
    let format = target.format;
    let pixel_size = format.pixel_size() as u32;
    let padded_bytes_per_row = aligned_bytes_per_row(width * pixel_size);

    for request in channels.request_rx.try_iter() {
        let buffer_size = padded_bytes_per_row as u64 * height as u64;
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("headless_screenshot_buffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("headless_screenshot_encoder"),
        });
        encoder.copy_texture_to_buffer(
            gpu_image.texture.as_image_copy(),
            ImageCopyBuffer {
                buffer: &buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        render_queue.submit([encoder.finish()]);

        let result = readback_buffer(
            &render_device,
            buffer,
            width,
            height,
            padded_bytes_per_row,
            pixel_size,
        )
        .and_then(|data| save_headless_image(data, width, height, format, &request.path));

        let _ = channels.result_tx.send(HeadlessScreenshotResult {
            path: request.path,
            error: result.err(),
        });
    }
}

pub fn create_headless_render_target(
    images: &mut Assets<Image>,
    config: HeadlessConfig,
) -> HeadlessRenderTarget {
    let size = Extent3d {
        width: config.size.x,
        height: config.size.y,
        depth_or_array_layers: 1,
    };
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("headless_render_target"),
            size,
            dimension: TextureDimension::D2,
            format: config.format,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        sampler: ImageSampler::nearest(),
        ..Default::default()
    };
    image.resize(size);
    let handle = images.add(image);
    HeadlessRenderTarget {
        image: handle,
        size: config.size,
        format: config.format,
    }
}

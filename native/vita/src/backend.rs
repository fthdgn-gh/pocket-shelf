//! Runs the core's draw commands with vita2d.

use std::collections::VecDeque;

use shelf_core::gfx::{Cmd, Color, Format, Texture, Textures};
use vita2d_sys::*;

#[derive(Clone, Copy)]
struct Gpu {
    /// Null once the app freed the texture.
    ptr: *mut vita2d_texture,
    /// The whole texture is uploaded; not drawn before.
    ready: bool,
    version: u32,
    w: u32,
    h: u32,
    format: Format,
}

/// Released textures kept for reuse, so a new picture of the same size
/// takes an old allocation instead of a fresh one. Vita3K faults when a live
/// app destroys textures repeatedly (src/CLAUDE.md); past this budget the
/// oldest are destroyed anyway.
const POOL_BUDGET: usize = 16 * 1024 * 1024;

/// Bytes copied into texture memory per frame from the upload queue (about
/// 2.5 ms on hardware).
const UPLOAD_BUDGET: usize = 256 * 1024;

/// A texture partly uploaded: rows before `row` are on the GPU.
struct Pending {
    index: usize,
    version: u32,
    row: u32,
}

#[derive(Default)]
pub struct Backend {
    gpu: Vec<Option<Gpu>>,
    pool: Vec<Gpu>,
    queue: VecDeque<Pending>,
}

/// What one `sync` uploaded.
#[derive(Default)]
pub struct Uploads {
    pub count: u32,
    pub bytes: usize,
}

/// vita2d colors are `0xAABBGGRR`.
fn abgr(c: Color) -> u32 {
    (c.a as u32) << 24 | (c.b as u32) << 16 | (c.g as u32) << 8 | c.r as u32
}

/// Copy rows `top..bottom` of a texture's pixels into its GPU memory;
/// returns the bytes copied.
unsafe fn copy_rows(ptr: *mut vita2d_texture, texture: &Texture, top: u32, bottom: u32) -> usize {
    let stride = vita2d_texture_get_stride(ptr) as usize;
    let dst = vita2d_texture_get_datap(ptr) as *mut u8;
    let row = texture.w as usize * texture.format.texel();
    for y in top as usize..bottom as usize {
        core::ptr::copy_nonoverlapping(texture.pixels.as_ptr().add(y * row), dst.add(y * stride), row);
    }
    row * bottom.saturating_sub(top) as usize
}

fn bytes(gpu: &Gpu) -> usize {
    (gpu.w * gpu.h) as usize * gpu.format.texel()
}

impl Backend {
    fn release(&mut self, gpu: Gpu) {
        if gpu.ptr.is_null() {
            return;
        }
        self.pool.push(gpu);
        while self.pool.iter().map(bytes).sum::<usize>() > POOL_BUDGET {
            let oldest = self.pool.remove(0);
            unsafe { vita2d_free_texture(oldest.ptr) };
        }
    }

    unsafe fn take(&mut self, w: u32, h: u32, format: Format) -> *mut vita2d_texture {
        if let Some(at) = self.pool.iter().position(|g| g.w == w && g.h == h && g.format == format) {
            return self.pool.remove(at).ptr;
        }
        let gxm = match format {
            Format::Rgba8 => SceGxmTextureFormat_SCE_GXM_TEXTURE_FORMAT_U8U8U8U8_ABGR,
            // The byte is alpha under white, so a tint colors it.
            Format::A8 => SceGxmTextureFormat_SCE_GXM_TEXTURE_FORMAT_U8_R111,
        };
        vita2d_create_empty_texture_format(w, h, gxm)
    }

    /// Upload new and changed textures, release freed ones.
    ///
    /// A texture marked `keep` (the glyph atlas) uploads at once, only its
    /// changed rows, since this frame draws from it. Any other texture
    /// uploads through a queue, at most `UPLOAD_BUDGET` bytes a frame: texture
    /// memory takes CPU writes at about 100 MB/s on hardware, so a 960×544
    /// backdrop in one go cost 21 ms. It is drawn once complete
    /// (`Texture::uploaded`), and its CPU copy is dropped then.
    ///
    /// # Safety
    /// Call between frames, after `vita2d_wait_rendering_done`: textures are
    /// written in place.
    pub unsafe fn sync(&mut self, textures: &mut Textures) -> Uploads {
        let mut uploads = Uploads::default();
        for index in 0..textures.list.len() {
            if index == self.gpu.len() {
                self.gpu.push(None);
            }
            let texture = &textures.list[index];
            if matches!(&self.gpu[index], Some(gpu) if gpu.version == texture.version) {
                continue;
            }
            let (w, h, format, version) = (texture.w, texture.h, texture.format, texture.version);
            let old = self.gpu[index].take();
            if texture.freed {
                if let Some(old) = old {
                    self.release(old);
                }
                self.gpu[index] = Some(Gpu { ptr: core::ptr::null_mut(), version, w, h, format, ready: false });
                continue;
            }
            let (ptr, same) = match old {
                Some(old) if !old.ptr.is_null() && old.w == w && old.h == h && old.format == format => (old.ptr, true),
                other => {
                    if let Some(old) = other {
                        self.release(old);
                    }
                    (self.take(w, h, format), false)
                }
            };
            self.gpu[index] = Some(Gpu { ptr, version, w, h, format, ready: false });
            let texture = &mut textures.list[index];
            texture.uploaded = false;
            if ptr.is_null() || texture.pixels.is_empty() {
                continue;
            }
            let filter = if texture.linear {
                SceGxmTextureFilter_SCE_GXM_TEXTURE_FILTER_LINEAR
            } else {
                SceGxmTextureFilter_SCE_GXM_TEXTURE_FILTER_POINT
            };
            vita2d_texture_set_filters(ptr, filter, filter);
            if texture.keep {
                // Only the changed rows, when the GPU copy is this texture's.
                let (top, bottom) = match texture.dirty {
                    Some((top, bottom)) if same => (top, bottom.min(h)),
                    _ => (0, h),
                };
                uploads.bytes += copy_rows(ptr, texture, top, bottom);
                uploads.count += 1;
                texture.dirty = Some((0, 0));
                texture.uploaded = true;
                if let Some(Some(gpu)) = self.gpu.get_mut(index) {
                    gpu.ready = true;
                }
            } else {
                self.queue.retain(|pending| pending.index != index);
                self.queue.push_back(Pending { index, version, row: 0 });
            }
        }

        // The queue, oldest first, within the frame's budget.
        let mut budget = UPLOAD_BUDGET;
        while budget > 0 {
            let Some(pending) = self.queue.front_mut() else { break };
            let index = pending.index;
            let texture = &mut textures.list[index];
            let current = matches!(&self.gpu[index], Some(gpu) if gpu.version == pending.version && !gpu.ptr.is_null());
            if !current || texture.version != pending.version || texture.pixels.is_empty() {
                self.queue.pop_front();
                continue;
            }
            let Some(Some(gpu)) = self.gpu.get_mut(index) else { break };
            let row_bytes = texture.w as usize * texture.format.texel();
            let rows = (budget / row_bytes).max(1) as u32;
            let bottom = (pending.row + rows).min(texture.h);
            let copied = copy_rows(gpu.ptr, texture, pending.row, bottom);
            uploads.bytes += copied;
            budget = budget.saturating_sub(copied);
            pending.row = bottom;
            if bottom == texture.h {
                uploads.count += 1;
                gpu.ready = true;
                texture.uploaded = true;
                texture.dirty = Some((0, 0));
                if !texture.keep {
                    texture.pixels = Vec::new();
                }
                self.queue.pop_front();
            }
        }
        textures.recycle_freed();
        uploads
    }

    /// # Safety
    /// Call inside `vita2d_start_drawing` / `vita2d_end_drawing`, after `sync`.
    pub unsafe fn run(&self, cmds: &[Cmd]) {
        for cmd in cmds {
            match *cmd {
                Cmd::Fill { rect, color } => vita2d_draw_rectangle(rect.x, rect.y, rect.w, rect.h, abgr(color)),
                Cmd::Image { tex, src, dst, tint } => {
                    let Some(Some(gpu)) = self.gpu.get(tex.0 as usize) else { continue };
                    if gpu.ptr.is_null() || !gpu.ready {
                        continue;
                    }
                    vita2d_draw_texture_tint_part_scale(
                        gpu.ptr,
                        dst.x,
                        dst.y,
                        src.x,
                        src.y,
                        src.w,
                        src.h,
                        dst.w / src.w,
                        dst.h / src.h,
                        abgr(tint),
                    );
                }
                Cmd::Clip(Some(r)) => {
                    vita2d_set_clip_rectangle(r.x as i32, r.y as i32, (r.x + r.w) as i32, (r.y + r.h) as i32);
                    vita2d_enable_clipping();
                }
                Cmd::Clip(None) => vita2d_disable_clipping(),
            }
        }
    }
}

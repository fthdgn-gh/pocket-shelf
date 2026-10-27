//! Frame timing: how long the CPU worked on a frame, and the time from one
//! frame's start to the next.

const WINDOW: usize = 120;

pub struct FrameStats {
    cpu: [u32; WINDOW],
    frame: [u32; WINDOW],
    at: usize,
    filled: usize,
    pub frames: u64,
    /// Frames longer than 20 ms since the last reset.
    pub slow: u64,
    pub longest_frame_us: u32,
    pub longest_cpu_us: u32,
}

impl Default for FrameStats {
    fn default() -> Self {
        FrameStats {
            cpu: [0; WINDOW],
            frame: [0; WINDOW],
            at: 0,
            filled: 0,
            frames: 0,
            slow: 0,
            longest_frame_us: 0,
            longest_cpu_us: 0,
        }
    }
}

impl FrameStats {
    pub fn push(&mut self, cpu_us: u32, frame_us: u32) {
        self.cpu[self.at] = cpu_us;
        self.frame[self.at] = frame_us;
        self.at = (self.at + 1) % WINDOW;
        self.filled = (self.filled + 1).min(WINDOW);
        self.frames += 1;
        if frame_us > 20_000 {
            self.slow += 1;
        }
        self.longest_frame_us = self.longest_frame_us.max(frame_us);
        self.longest_cpu_us = self.longest_cpu_us.max(cpu_us);
    }

    pub fn reset(&mut self) {
        *self = FrameStats::default();
    }

    fn average(values: &[u32], filled: usize) -> f32 {
        if filled == 0 {
            return 0.0;
        }
        values[..filled].iter().map(|&v| v as f32).sum::<f32>() / filled as f32 / 1000.0
    }

    /// Averages over the last two seconds.
    pub fn cpu_ms(&self) -> f32 {
        Self::average(&self.cpu, self.filled)
    }

    pub fn frame_ms(&self) -> f32 {
        Self::average(&self.frame, self.filled)
    }

    /// The on-screen form: CPU and frame averages and maximums, slow frames.
    pub fn short(&self) -> String {
        format!(
            "CPU {:.1}/{:.1}  frame {:.1}/{:.1}  slow {}",
            self.cpu_ms(),
            self.longest_cpu_us as f32 / 1000.0,
            self.frame_ms(),
            self.longest_frame_us as f32 / 1000.0,
            self.slow
        )
    }

    pub fn line(&self) -> String {
        format!(
            "CPU {:.1} ms (max {:.1})  frame {:.1} ms (max {:.1})  over 20 ms: {}",
            self.cpu_ms(),
            self.longest_cpu_us as f32 / 1000.0,
            self.frame_ms(),
            self.longest_frame_us as f32 / 1000.0,
            self.slow
        )
    }
}

/// Where one frame's time went, for the slow-frame log. Microseconds per
/// phase, plus what the frame did that can cost time.
#[derive(Clone, Copy, Default, Debug)]
pub struct Trace {
    /// Waiting for the GPU to finish the previous frame.
    pub wait: u32,
    pub events: u32,
    pub event_count: u32,
    pub update: u32,
    pub draw: u32,
    /// Texture uploads.
    pub sync: u32,
    pub uploads: u32,
    pub upload_kb: u32,
    /// Handing the commands to the GPU.
    pub submit: u32,
    pub commands: u32,
    pub glyphs: u32,
    pub textures_added: u32,
}

impl Trace {
    pub fn line(&self, frame: u64, frame_us: u32, cpu_us: u32) -> String {
        let ms = |us: u32| us as f32 / 1000.0;
        format!(
            "slow frame {frame}: {:.1} ms (cpu {:.1}) | wait {:.1} events {:.1} ({}) update {:.1} draw {:.1} ({} cmds, {} glyphs, {} textures) sync {:.1} ({} uploads, {} KB) submit {:.1}",
            ms(frame_us),
            ms(cpu_us),
            ms(self.wait),
            ms(self.events),
            self.event_count,
            ms(self.update),
            ms(self.draw),
            self.commands,
            self.glyphs,
            self.textures_added,
            ms(self.sync),
            self.uploads,
            self.upload_kb,
            ms(self.submit),
        )
    }
}

//! The pickers' sample preview (the Sample strip): one sample played once from a frame, at gain 0.7, centred, at its
//! own pitch (rate 1), no loop. It goes out past the master and its meter, as the click does, so no channel, pad or
//! effect applies (master effects neither).

use std::sync::Arc;

use honeydew_audio::sample::SampleData;
use honeydew_audio::stream::PreviewStream;

/// The preview's gain.
pub const GAIN: f32 = 0.7;

enum Sample {
    Memory(Arc<SampleData>),
    Stream(Arc<PreviewStream>),
}

pub(super) struct Preview {
    sample: Sample,
    /// Where it is in the sample's frames, and how far it moves an output frame.
    pos: f64,
    step: f64,
}

impl Drop for Preview {
    fn drop(&mut self) {
        if let Sample::Stream(sample) = &self.sample {
            sample.stop();
        }
    }
}

impl Preview {
    /// `data` from frame `from`, at output rate `rate`.
    pub fn new(data: Arc<SampleData>, from: usize, rate: f32) -> Self {
        let step = f64::from(data.sample_rate) / f64::from(rate);
        Self {
            sample: Sample::Memory(data),
            pos: from as f64,
            step,
        }
    }

    /// `sample` from frame `from`, at output rate `rate`.
    pub fn streaming(sample: Arc<PreviewStream>, from: usize, rate: f32) -> Self {
        sample.start(from);
        let step = f64::from(sample.sample_rate()) / f64::from(rate);
        Self {
            sample: Sample::Stream(sample),
            pos: from as f64,
            step,
        }
    }

    /// Adds its next frames into `left` and `right` (linear between frames); false once it is past the end.
    pub fn add(&mut self, left: &mut [f32], right: &mut [f32]) -> bool {
        match &self.sample {
            Sample::Memory(data) => {
                let (l, r) = (&*data.left, data.right.as_deref());
                for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
                    let i = self.pos as usize;
                    if i >= l.len() || !self.step.is_finite() || self.step <= 0.0 {
                        return false;
                    }
                    let t = (self.pos - i as f64) as f32;
                    let at = |s: &[f32]| {
                        s[i] + (s.get(i + 1).copied().unwrap_or(0.0) - s[i]) * t
                    };
                    let a = at(l);
                    let b = r.map_or(a, at);
                    *out_l += GAIN * a;
                    *out_r += GAIN * b;
                    self.pos += self.step;
                }
                (self.pos as usize) < l.len()
            }
            Sample::Stream(sample) => {
                for (out_l, out_r) in left.iter_mut().zip(right.iter_mut()) {
                    let i = self.pos as usize;
                    if i >= sample.frames() || !self.step.is_finite() || self.step <= 0.0 {
                        return false;
                    }
                    let Some((a, b)) = sample.frame(i) else {
                        sample.underrun();
                        continue;
                    };
                    let t = (self.pos - i as f64) as f32;
                    let next = sample.frame(i + 1).unwrap_or((a, b));
                    *out_l += GAIN * (a + (next.0 - a) * t);
                    *out_r += GAIN * (b + (next.1 - b) * t);
                    self.pos += self.step;
                    sample.advance(i);
                }
                (self.pos as usize) < sample.frames()
            }
        }
    }
}

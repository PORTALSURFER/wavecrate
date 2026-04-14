use std::f32::consts::PI;
use std::sync::Arc;

/// Complex sample type used by FFT helpers.
pub use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};

/// Build a Hann window with `length` samples.
pub fn hann_window(length: usize) -> Vec<f32> {
    if length <= 1 {
        return vec![1.0_f32; length.max(1)];
    }
    let denom = (length - 1) as f32;
    (0..length)
        .map(|n| 0.5_f32 * (1.0 - (2.0 * PI * n as f32 / denom).cos()))
        .collect()
}

#[cfg(test)]
pub(crate) fn fft_radix2_inplace(buffer: &mut [Complex32]) -> Result<(), String> {
    let plan = FftPlan::new(buffer.len())?;
    fft_radix2_inplace_with_plan(buffer, &plan)
}

/// Reusable radix-2 FFT plan.
pub struct FftPlan {
    len: usize,
    plan: Arc<dyn Fft<f32>>,
}

impl FftPlan {
    /// Create a forward FFT plan for a power-of-two buffer length.
    pub fn new(len: usize) -> Result<Self, String> {
        if len == 0 || !len.is_power_of_two() {
            return Err(format!("FFT length must be power-of-two, got {len}"));
        }
        let mut planner = FftPlanner::<f32>::new();
        Ok(Self {
            len,
            plan: planner.plan_fft_forward(len),
        })
    }
}

/// Run an in-place forward radix-2 FFT using a cached plan.
pub fn fft_radix2_inplace_with_plan(
    buffer: &mut [Complex32],
    plan: &FftPlan,
) -> Result<(), String> {
    if buffer.len() != plan.len {
        return Err(format!(
            "FFT length mismatch: buffer {} plan {}",
            buffer.len(),
            plan.len
        ));
    }
    plan.plan.process(buffer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_window_is_symmetric_and_zero_at_edges() {
        let w = hann_window(8);
        assert!((w[0]).abs() < 1e-6);
        assert!((w[7]).abs() < 1e-6);
        assert!((w[1] - w[6]).abs() < 1e-6);
    }

    #[test]
    fn fft_produces_expected_bin_for_constant_signal() {
        let mut buf = vec![Complex32::new(1.0, 0.0); 8];
        fft_radix2_inplace(&mut buf).unwrap();
        assert!((buf[0].re - 8.0).abs() < 1e-4);
        for value in buf.iter().take(8).skip(1) {
            assert!(value.re.abs() < 1e-4);
            assert!(value.im.abs() < 1e-4);
        }
    }

    #[test]
    fn fft_plan_matches_plain_fft() {
        let mut buf = vec![Complex32::new(0.0, 0.0); 16];
        for (i, cell) in buf.iter_mut().enumerate() {
            cell.re = (i as f32 * 0.25).sin();
        }
        let mut planned = buf.clone();
        fft_radix2_inplace(&mut buf).unwrap();
        let plan = FftPlan::new(planned.len()).unwrap();
        fft_radix2_inplace_with_plan(&mut planned, &plan).unwrap();
        for i in 0..buf.len() {
            assert!((buf[i].re - planned[i].re).abs() < 1e-4);
            assert!((buf[i].im - planned[i].im).abs() < 1e-4);
        }
    }
}

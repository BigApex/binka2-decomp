#![allow(unused_imports)]

use rustdct::algorithm::Type2And3ConvertToFft;
use rustdct::rustfft::{num_complex::Complex, FftPlanner};
use rustdct::Dct3;

pub fn idct(
    output: &mut [u16],
    transform_ratio: f32,
    // transform: &mut [f32],
    coeffs: &[f32],
    transform_size: u32,
) {
    debug_assert!(transform_size.is_power_of_two());

    // This version is from crate rustfft
    // It has a cut right before the end of the spectrogram (high)
    // Game's version has about the same window but it has empty space at the top

    // let mut transform = vec![0f32; transform_size as usize];
    
    // let mut planner = FftPlanner::new();
    // let fft = planner.plan_fft_forward(transform_size as usize);
    // let dct = Type2And3ConvertToFft::new(fft);

    // transform.copy_from_slice(coeffs);
    // dct.process_dct3(&mut transform);

    // todo!("DCT3 with sqrt(2) shit and not 2");

    // Hand-written dct3 - looks smoother and more complete compared to game version
    // What not to say about the other one

    const TWIDDLE_MIDPOINT: usize = 512;
    let twiddle_shift = (transform_size/2) - 512;
    // println!("TS: {}", twiddle_shift);

    let angle_constant_1 = std::f32::consts::PI * -2f32 / 4096f32;
    let angle_constant_2 = std::f32::consts::PI * -2f32 / 8912f32;
    let twiddles = (twiddle_shift as usize..transform_size as usize)
        .map(|i| if i < TWIDDLE_MIDPOINT {
            Complex::from_polar(1f32, angle_constant_1 * i as f32)
        } else {
            Complex::from_polar(1f32, angle_constant_2 * (i-TWIDDLE_MIDPOINT) as f32)
        })
        .collect::<Vec<Complex<f32>>>();

    // modulation
    let mut transform = vec![Complex::<f32>::default(); transform_size as usize / 2];
    {
        #[allow(clippy::excessive_precision)]
        const FACTOR: f32 = 1.4142135623730950488f32;
        transform[0].re = coeffs[0] + coeffs[0];
        transform[0].im = FACTOR * coeffs[transform_size as usize / 2];

        for i in 1..transform_size as usize / 2 {
            let a = (coeffs[i], coeffs[transform_size as usize - i]);
            transform[i].re = twiddles[i].re * a.0 - twiddles[i].im * a.1;
            transform[i].im = twiddles[i].re * a.1 + twiddles[i].im * a.0;
        }
    }

    // idk
    let tmp = transform[0];
    transform[0].re = (tmp.re + tmp.im) / 2f32;
    transform[0].im = (tmp.re - tmp.im) / 2f32;

    if transform_size/4 > 0 {
        let angle_constant = std::f32::consts::PI * -2f32 / transform_size as f32;
        let twiddles = (0..transform_size as usize / 4)
            .map(|i| Complex::from_polar(1f32, angle_constant * i as f32))
            .collect::<Vec<Complex<f32>>>();
        
        for i in 1..transform_size as usize / 4 {
            let back_idx = (transform_size as usize / 2) - i;

            let t_f = transform[i];
            let t_b = transform[back_idx];
            
            let half_r = (t_f.re - t_b.re) / 2f32;
            let half_i = (t_f.im - t_b.im) / 2f32;
            
            let a = t_f.re - half_r;
            let b = t_f.im - half_i;

            let c = twiddles[i].re*half_i + twiddles[i].im*half_r;
            let d = twiddles[i].re*half_r - twiddles[i].im*half_i;

            transform[i].re = a - c;
            transform[i].im = b + d;

            transform[back_idx].re = a + c;
            transform[back_idx].im = d - b; // what?
        }
    }

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(transform_size as usize / 2);
    fft.process(&mut transform);

    let transform_scalar = unsafe { std::slice::from_raw_parts(transform.as_ptr().cast::<f32>(), transform_size as usize) };

    for i in 0..(transform_size as usize / 2) {
        let a = transform_scalar[i] * transform_ratio;
        let b = transform_scalar[transform_scalar.len() - i - 1] * transform_ratio;

        let a = a as i32;
        let b = b as i32;

        let a = a.min(32767).max(-32768);
        let b = b.min(32767).max(-32768);

        let a = a as i16 as u16;
        let b = b as i16 as u16;
        output[(i * 2)] = a;
        output[(i * 2) + 1] = b;
    }

}

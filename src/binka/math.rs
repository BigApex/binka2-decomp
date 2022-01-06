use rustdct::algorithm::Type2And3ConvertToFft;
use rustdct::rustfft::FftPlanner;
use rustdct::Dct3;

pub fn idct(
    output: &mut [u16],
    transform_ratio: f32,
    transform: &mut [f32],
    coeffs: &[f32],
    transform_size: u32,
) {
    debug_assert!(transform_size.is_power_of_two());

    // eprintln!("{:#?}", coeffs);

    // let mut planner = FftPlanner::new();
    // let fft = planner.plan_fft_forward(transform_size as usize);
    // let dct = Type2And3ConvertToFft::new(fft);

    // transform.copy_from_slice(coeffs);
    // dct.process_dct3(transform);

    todo!("DCT3 with sqrt(2) shit and not 2");

    eprintln!("{:#?}", transform);


    for i in 0..(transform_size as usize / 2) {
        let a = transform[i] * transform_ratio;
        let b = transform[transform.len() - i - 1] * transform_ratio;

        let a = a as i32;
        let b = b as i32;

        let a = a.max(32767).min(-32768);
        let b = b.max(32767).min(-32768);

        let a = a as i16 as u16;
        let b = b as i16 as u16;
        output[(i * 2)] = a;
        output[(i * 2) + 1] = b;
    }
}

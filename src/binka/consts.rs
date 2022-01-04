pub const BINKA2_FLAG_V2: u32 = 4;
pub const BINKA2_FLAG_NOT_ONE_CHAN: u32 = 2; // set only when 1 channel is present
pub const BINKA2_FLAG_IDK: u32 = 1;
pub const BINKA2_BANDS_MAX: usize = 0x19;

// These were provided by people who had SDK
pub const BINKA2_FLOAT_CONST: f32 = 1.41421356237309504880;
const BINKA2_TRANSFORMS_INTERNAL: [f32; 4] = [
    2f32 / 64f32,                        // 2048
    2f32 / (32f32 * BINKA2_FLOAT_CONST), // 2048, 1024
    2f32 / 32f32,                        // 1024, 2048
    2f32 / (16f32 * BINKA2_FLOAT_CONST), // 1024
];
// first big then small
pub const BINKA2_TRANSFORMS: [(u32, f32, f32); 3] = [
    (
        2048,
        BINKA2_TRANSFORMS_INTERNAL[1],
        BINKA2_TRANSFORMS_INTERNAL[0],
    ),
    (
        1024,
        BINKA2_TRANSFORMS_INTERNAL[2],
        BINKA2_TRANSFORMS_INTERNAL[1],
    ),
    (
        512,
        BINKA2_TRANSFORMS_INTERNAL[3],
        BINKA2_TRANSFORMS_INTERNAL[2],
    ),
];
pub const BINKA2_CRIT_FREQS: [u32; BINKA2_BANDS_MAX] = [
    0, 100, 200, 300, 400, 510, 630, 770, 920, 1080, 1270, 1480, 1720, 2000, 2320, 2700, 3150,
    3700, 4400, 5300, 6400, 7700, 9500, 12000, 15500,
];

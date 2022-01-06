pub const BINKA2_FLAG_V2: u32 = 4; // hot stuff unused in TF|2 and Apex's MSS?
pub const BINKA2_FLAG_NOT_ONE_CHAN: u32 = 2; // set only when 1 channel is present
pub const BINKA2_FLAG_DCT: u32 = 1; // newer version-ish?
pub const BINKA2_BANDS_MAX: usize = 0x19;
pub const BINKA2_FIXED_FLOAT_BITS: usize = 29;

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

pub const BINKA2_RLE: [u32; 16] = [2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 32, 64];

// константы квантов которые равны 10**(i*0.066399999) с лютой погрешностью, но так декодит сам бинкавин
pub const BINKA2_QUANTS: [f32; 96] = [
    1.0,
    1.1651986837387085,
    1.357688069343567,
    1.5819764137268066,
    1.843316912651062,
    2.1478304862976074,
    2.5026493072509766,
    2.916083812713623,
    3.3978171348571777,
    3.959132194519043,
    4.613175868988037,
    5.3752665519714355,
    6.263253688812256,
    7.2979350090026855,
    8.503544807434082,
    9.908319473266602,
    11.545161247253418,
    13.452406883239746,
    15.674727439880371,
    18.264171600341797,
    21.281391143798828,
    24.797048568725586,
    28.893489837646484,
    33.666656494140625,
    39.22834777832031,
    45.70882034301758,
    53.259857177734375,
    62.058319091796875,
    72.31027221679688,
    84.2558364868164,
    98.17479705810547,
    114.39314270019531,
    133.29074096679688,
    155.31021118164062,
    180.96725463867188,
    210.86280822753906,
    245.69708251953125,
    286.2859191894531,
    333.5799865722656,
    388.6869812011719,
    452.8975830078125,
    527.7156982421875,
    614.8936157226562,
    716.4732666015625,
    834.833740234375,
    972.7472534179688,
    1133.44384765625,
    1320.687255859375,
    1538.8631591796875,
    1793.0814208984375,
    2089.296142578125,
    2434.445068359375,
    2836.6123046875,
    3305.21728515625,
    3851.23486328125,
    4487.4541015625,
    5228.775390625,
    6092.5625,
    7099.04638671875,
    8271.7998046875,
    9638.2900390625,
    11230.5234375,
    13085.7919921875,
    15247.5478515625,
    17766.423828125,
    20701.4140625,
    24121.259765625,
    28106.0625,
    32749.1484375,
    38159.265625,
    44463.125,
    51808.37890625,
    60367.0546875,
    70339.6171875,
    81959.6328125,
    95499.2578125,
    111275.6171875,
    129658.203125,
    151077.578125,
    176035.390625,
    205116.21875,
    239001.15625,
    278483.84375,
    324489.03125,
    378094.1875,
    440554.875,
    513333.96875,
    598136.0625,
    696947.375,
    812082.1875,
    946237.1875,
    1102554.375,
    1284694.875,
    1496924.875,
    1744214.875,
    2032357.0,
];

pub const BINKA2_FXP_POW: [f32; 24] = [
    (1f32 / (1 << 23) as f32),
    (1f32 / (1 << 22) as f32),
    (1f32 / (1 << 21) as f32),
    (1f32 / (1 << 20) as f32),
    (1f32 / (1 << 19) as f32),
    (1f32 / (1 << 18) as f32),
    (1f32 / (1 << 17) as f32),
    (1f32 / (1 << 16) as f32),
    (1f32 / (1 << 15) as f32),
    (1f32 / (1 << 14) as f32),
    (1f32 / (1 << 13) as f32),
    (1f32 / (1 << 12) as f32),
    (1f32 / (1 << 11) as f32),
    (1f32 / (1 << 10) as f32),
    (1f32 / (1 << 9) as f32),
    (1f32 / (1 << 8) as f32),
    (1f32 / (1 << 7) as f32),
    (1f32 / (1 << 6) as f32),
    (1f32 / (1 << 5) as f32),
    (1f32 / (1 << 4) as f32),
    (1f32 / (1 << 3) as f32),
    (1f32 / (1 << 2) as f32),
    (1f32 / (1 << 1) as f32),
    (1f32 / (1 << 0) as f32),
];

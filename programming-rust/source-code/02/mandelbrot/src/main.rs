use image::png::PNGEncoder;
use image::ColorType;
use num::Complex;
use std::env;
use std::fs::File;
use std::str::FromStr;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} FILE PIXELS UPPERLEFT LOWERRIGHT", args[0]);
        eprintln!(
            "Example: {} mandel.png 1000x750 -1.20,0.35 -1,0.20",
            args[0]
        );
        std::process::exit(1);
    }

    let bounds = parse_pair(&args[2], 'x').expect("解析图像尺寸出错");
    let upper_left = parse_complex(&args[3]).expect("解析左上角点出错");
    let lower_right = parse_complex(&args[4]).expect("解析右下角点出错");

    let mut pixels = vec![0; bounds.0 * bounds.1];

    // render(&mut pixels, bounds, upper_left, lower_right);
    let threads = 8;
    let rows_per_band = bounds.1 / threads + 1;

    {
        let bands: Vec<&mut [u8]> = pixels.chunks_mut(rows_per_band * bounds.0).collect();
        crossbeam::scope(|spawner| {
            for (i, band) in bands.into_iter().enumerate() {
                let top = rows_per_band * i;
                let height = band.len() / bounds.0;
                let band_bounds = (bounds.0, height);
                let band_upper_left = pixel_to_point(bounds, (0, top), upper_left, lower_right);
                let band_lower_right =
                    pixel_to_point(bounds, (bounds.0, top + height), upper_left, lower_right);

                spawner.spawn(move |_| {
                    render(band, band_bounds, band_upper_left, band_lower_right);
                });
            }
        })
        .unwrap();
    }

    write_image(&args[1], &pixels, bounds).expect("写入PNG文件出错");
}

/// 尝试决定`c`是否位于Mandelbrot集中，最多进行`limit`次来作出决策。
///
/// 如果`c`不是成员，返回`Some(i)`，其中`i`为`c`离开以原点为中心半径为2区域所需的次数。
/// 如果`c`是成员（更确切的说是如果迭代了limit次后还无法证明`c`不是其成员），返回`None`。
fn escape_time(c: Complex<f64>, limit: usize) -> Option<usize> {
    let mut z = Complex { re: 0.0, im: 0.0 };
    for i in 0..limit {
        if z.norm_sqr() > 4.0 {
            return Some(i);
        }
        z = z * z + c;
    }

    None
}

/// 将字符串`s`解析为坐标对，如`"400x600"`或`"1.0,0.5"`。
///
/// 具体来说，`s`的形式就为<left><sep><right>，其中<sep>是由`separator`所给定的字符，
/// <left>和<right> 均是字符串，可由`T::from_str`解析。`separator`必须是ASCII字符。
///
/// 如果`s`格式正确，返回`Some<(x, y)>`。如解析错误，返回`None`。
fn parse_pair<T: FromStr>(s: &str, separator: char) -> Option<(T, T)> {
    match s.find(separator) {
        None => None,
        Some(index) => match (T::from_str(&s[..index]), T::from_str(&s[index + 1..])) {
            (Ok(l), Ok(r)) => Some((l, r)),
            _ => None,
        },
    }
}

#[test]
fn test_parse_pair() {
    assert_eq!(parse_pair::<i32>("", ','), None);
    assert_eq!(parse_pair::<i32>("10,", ','), None);
    assert_eq!(parse_pair::<i32>("10,20", ','), Some((10, 20)));
    assert_eq!(parse_pair::<i32>("10,20xy", ','), None);
    assert_eq!(parse_pair::<f64>("0.5x", 'x'), None);
    assert_eq!(parse_pair::<f64>("0.5x1.5", 'x'), Some((0.5, 1.5)));
}

/// 解析一对以逗号分隔的浮点数值为复数。
fn parse_complex(s: &str) -> Option<Complex<f64>> {
    match parse_pair(s, ',') {
        Some((re, im)) => Some(Complex { re, im }),
        None => None,
    }
}

#[test]
fn test_parse_complex() {
    assert_eq!(
        parse_complex("1.25,-0.0625"),
        Some(Complex {
            re: 1.25,
            im: -0.0625
        })
    );
    assert_eq!(parse_complex(",-0.0625"), None);
}

/// 给定输出图像中像素的行列，返回复数平面中对应的点。
///
/// `bounds`按像素给定图像的宽高。
/// `pixel`表示图像中具体像素的(column, row)对。
/// `upper_left`和`lower_right`参数指向指定图像区域的复数平面的点。
fn pixel_to_point(
    bounds: (usize, usize),
    pixel: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) -> Complex<f64> {
    let (width, height) = (
        lower_right.re - upper_left.re,
        upper_left.im - lower_right.im,
    );
    Complex {
        re: upper_left.re + pixel.0 as f64 * width / bounds.0 as f64,
        im: upper_left.im - pixel.1 as f64 * height / bounds.1 as f64,
        // 为什么在这里减？pixel.1越往下越大，但虚部越往上越大。
    }
}

#[test]
fn test_pixel_to_point() {
    assert_eq!(
        pixel_to_point(
            (100, 200),
            (25, 175),
            Complex { re: -1.0, im: 1.0 },
            Complex { re: 1.0, im: -1.0 }
        ),
        Complex { re: -0.5, im: 0.75 }
    );
}

/// 将Mandelbrot集的矩形渲染为像素缓冲。
///
/// `bounds`参数给定了`pixels`缓冲的宽和高，缓冲中按字节存储了相素灰度。
/// `upper_left`和`lower_right`指定与像素缓冲左上角和右下角对应的复数平面。
fn render(
    pixels: &mut [u8],
    bounds: (usize, usize),
    upper_left: Complex<f64>,
    lower_right: Complex<f64>,
) {
    assert!(pixels.len() == bounds.0 * bounds.1);

    for row in 0..bounds.1 {
        for column in 0..bounds.0 {
            let point = pixel_to_point(bounds, (column, row), upper_left, lower_right);
            pixels[row * bounds.0 + column] = match escape_time(point, 255) {
                None => 0,
                Some(count) => 255 - count as u8,
            };
        }
    }
}

/// 写缓冲`pixels`，大小由`bounds`指定, 文件名为`filename`。
fn write_image(
    filename: &str,
    pixels: &[u8],
    bounds: (usize, usize),
) -> Result<(), std::io::Error> {
    let output = File::create(filename)?;

    let encoder = PNGEncoder::new(output);
    encoder.encode(pixels, bounds.0 as u32, bounds.1 as u32, ColorType::Gray(8))?;

    Ok(())
}

// fn square_loop(mut x: f64) {
//     loop {
//         x = x * x;
//     }
// }

// fn square_add_loop(c: f64) {
//     let mut x = 0.;
//     loop {
//         x = x * x + c;
//     }
// }

// fn complex_square_add_loop(c: Complex<f64>) {
//     let mut z = Complex { re: 0.0, im: 0.0 };
//     loop {
//         z = z * z + c;
//     }
// }

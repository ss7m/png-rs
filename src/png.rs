#![allow(dead_code)]
use libc::{c_char, c_int, c_void, fclose, fopen, fread, size_t, FILE};
use std::ffi::CString;
use std::ptr;
use std::slice;

const PNG_TRANSFORM_IDENTITY: c_int = 0;
const PNG_TRANSFORM_STRIP_16: c_int = 1;
const PNG_TRANSFORM_PACKING: c_int = 4;

const PNG_COLOR_MASK_PALETTE: u8 = 1;
const PNG_COLOR_MASK_COLOR: u8 = 2;
const PNG_COLOR_MASK_ALPHA: u8 = 4;

const PNG_COLOR_TYPE_GRAY: u8 = 0;
const PNG_COLOR_TYPE_PALETTE: u8 = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
const PNG_COLOR_TYPE_RGB: u8 = PNG_COLOR_MASK_COLOR;
const PNG_COLOR_TYPE_RGB_ALPHA: u8 = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
const PNG_COLOR_TYPE_GRAY_ALPHA: u8 = PNG_COLOR_MASK_ALPHA;

const PNG_INTERLACE_NONE: c_int = 0;
const PNG_COMPRESSION_TYPE_DEFAULT: c_int = 0;
const PNG_FILTER_TYPE_DEFAULT: c_int = 0;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ColorType {
    Gray(),
    RGB(),
    RGBAlpha(),
    GrayAlpha(),
}

impl ColorType {
    fn num_channels(self) -> u8 {
        match self {
            ColorType::Gray() => 1,
            ColorType::RGB() => 3,
            ColorType::RGBAlpha() => 4,
            ColorType::GrayAlpha() => 1,
        }
    }

    fn as_c_int(self) -> c_int {
        (match self {
            ColorType::Gray() => PNG_COLOR_TYPE_GRAY,
            ColorType::RGB() => PNG_COLOR_TYPE_RGB,
            ColorType::RGBAlpha() => PNG_COLOR_TYPE_RGB_ALPHA,
            ColorType::GrayAlpha() => PNG_COLOR_TYPE_GRAY_ALPHA,
        }) as c_int
    }
}

#[allow(non_camel_case_types)]
type c_png_struct = c_void;

#[allow(non_camel_case_types)]
type c_png_info = c_void;

#[link(name = "png")]
extern "C" {
    fn png_sig_cmp(sig: *const u8, start: size_t, num_to_check: size_t) -> c_int;
    fn png_set_sig_bytes(png_struct: *mut c_png_struct, num_bytes: c_int);
    fn png_create_read_struct(
        version: *const c_char,
        error_ptr: *mut u8,
        error_fn: *mut u8,
        warning_fn: *mut u8,
    ) -> *mut c_png_struct;

    fn png_create_write_struct(
        version: *const c_char,
        error_ptr: *mut u8,
        error_fn: *mut u8,
        warning_fn: *mut u8,
    ) -> *mut c_png_struct;

    fn png_create_info_struct(png_struct: *mut c_png_struct) -> *mut c_png_info;

    fn png_destroy_read_struct(
        png_structpp: *mut *mut c_png_struct,
        png_infopp: *mut *mut c_png_info,
        png_endpp: *mut *mut c_png_info,
    );

    fn png_init_io(png_struct: *mut c_png_struct, file: *mut FILE);

    fn png_read_png(
        png_sturct: *mut c_png_struct,
        png_info: *mut c_png_info,
        transforms: c_int,
        params: *mut c_void,
    );

    fn png_get_image_width(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u32;
    fn png_get_image_height(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u32;
    fn png_get_bit_depth(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u8;
    fn png_get_color_type(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> u8;
    fn png_get_rows(png_struct: *mut c_png_struct, png_info: *mut c_png_info) -> *mut *mut u8;

    fn png_set_rows(png_struct: *mut c_png_struct, png_info: *mut c_png_info, rows: *mut *mut u8);

    fn png_set_IHDR(
        png_struct: *mut c_png_struct,
        png_info: *mut c_png_info,
        width: u32,
        height: u32,
        bit_depth: c_int,
        color_type: c_int,
        interlace_type: c_int,
        compression_type: c_int,
        filter_method: c_int,
    );

    fn png_write_png(
        png_struct: *mut c_png_struct,
        png_info: *mut c_png_info,
        transforms: c_int,
        params: *mut c_void,
    );
}

fn check_if_png(file: *mut FILE) -> bool {
    let mut bytes: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
    let bytes = bytes.as_mut_ptr();
    unsafe {
        fread(bytes as *mut c_void, 1, 8, file);
        png_sig_cmp(bytes, 0, 8) == 0
    }
}

pub struct PNG {
    png_struct: *mut c_png_struct,
    png_info: *mut c_png_info,
    filep: Option<*mut FILE>,
}

impl Drop for PNG {
    fn drop(&mut self) {
        unsafe {
            png_destroy_read_struct(&mut self.png_struct, &mut self.png_info, ptr::null_mut())
        };
        if let Some(filep) = self.filep {
            if !filep.is_null() {
                unsafe { fclose(filep) };
            }
        }
    }
}

impl PNG {
    pub fn new(file_name: &str) -> Result<PNG, String> {
        let png_struct = unsafe {
            let version = CString::new("1.6.37").expect("CString::new failed");
            png_create_read_struct(
                version.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if png_struct.is_null() {
            return Err("Error creating png struct".to_string());
        }

        let png_info = unsafe { png_create_info_struct(png_struct) };

        if png_info.is_null() {
            return Err("Error creating onfo struct".to_string());
        }

        let mut png = PNG {
            png_struct,
            png_info,
            filep: None,
        };

        if png.read_file(file_name) {
            Ok(png)
        } else {
            Err(format!("{} is not a png file or does not exist", file_name))
        }
    }

    // TODO: make a separate struct for writing png?
    fn write_png(file_name: &str) -> Result<PNG, String> {
        let png_struct = unsafe {
            let version = CString::new("1.6.37").expect("CString::new failed");
            png_create_write_struct(
                version.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        if png_struct.is_null() {
            return Err("Error creating png struct".to_string());
        }

        let png_info = unsafe { png_create_info_struct(png_struct) };

        if png_info.is_null() {
            return Err("Error getting png info".to_string());
        }

        let filep = unsafe {
            let c_file_name = CString::new(file_name).expect("CString::new failed");
            let mode = CString::new("wb").expect("CString::new failed");
            fopen(c_file_name.as_ptr(), mode.as_ptr())
        };

        if filep.is_null() {
            return Err(format!("Error opening file for writing: {}", file_name));
        }

        unsafe { png_init_io(png_struct, filep) };

        Ok(PNG {
            png_struct,
            png_info,
            filep: Some(filep),
        })
    }

    pub fn get_width(&self) -> u32 {
        unsafe { png_get_image_width(self.png_struct, self.png_info) }
    }

    pub fn get_height(&self) -> u32 {
        unsafe { png_get_image_height(self.png_struct, self.png_info) }
    }

    // should always return 8
    pub fn get_bit_depth(&self) -> u8 {
        unsafe { png_get_bit_depth(self.png_struct, self.png_info) }
    }

    pub fn get_color_type(&self) -> ColorType {
        let color_type = unsafe { png_get_color_type(self.png_struct, self.png_info) };
        match color_type {
            PNG_COLOR_TYPE_GRAY => ColorType::Gray(),
            PNG_COLOR_TYPE_RGB => ColorType::RGB(),
            PNG_COLOR_TYPE_RGB_ALPHA => ColorType::RGBAlpha(),
            PNG_COLOR_TYPE_GRAY_ALPHA => ColorType::GrayAlpha(),
            PNG_COLOR_TYPE_PALETTE => panic!("Palette not supported"),
            _ => panic!("Unrecognized color type: {}", color_type),
        }
    }

    fn read_file(&mut self, file_name: &str) -> bool {
        // Currently this coerces all channels to 8 bits
        let filep = unsafe {
            let file_name = CString::new(file_name).expect("CString::new failed");
            let mode = CString::new("rb").expect("CString::new failed");
            fopen(file_name.as_ptr(), mode.as_ptr())
        };

        if filep.is_null() {
            return false;
        } else if !check_if_png(filep) {
            unsafe { fclose(filep) };
            return false;
        }

        self.filep = Some(filep);

        unsafe {
            png_init_io(self.png_struct, filep);
            png_set_sig_bytes(self.png_struct, 8);
            png_read_png(
                self.png_struct,
                self.png_info,
                PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_PACKING,
                ptr::null_mut(),
            );
        }

        true
    }

    pub fn get_image(&self) -> Image {
        let color_type = self.get_color_type();
        let num_channels = color_type.num_channels();
        //let bit_depth = self.get_bit_depth();

        let rows = unsafe {
            let rows = png_get_rows(self.png_struct, self.png_info);
            slice::from_raw_parts(rows, self.get_height() as usize)
        };
        let mut rows_vec = Vec::new();
        let row_size = (self.get_width() * (num_channels as u32)) as usize;
        for &row in rows {
            for item in unsafe { slice::from_raw_parts(row, row_size).to_vec() } {
                rows_vec.push(item);
            }
        }

        Image {
            height: self.get_height() as usize,
            width: self.get_width() as usize,
            color_type,
            data: rows_vec,
        }
    }
}

#[derive(Clone)]
pub struct Image {
    pub height: usize,
    pub width: usize,
    pub color_type: ColorType,
    pub data: Vec<u8>,
}

impl Image {
    fn row_size(&self) -> usize {
        self.width * (self.color_type.num_channels() as usize)
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Pixel {
        let row_size = self.row_size();

        match self.color_type {
            ColorType::Gray() => Pixel::Gray(self.data[y * row_size + x]),
            ColorType::RGB() => {
                let red = self.data[y * row_size + x * 3];
                let green = self.data[y * row_size + x * 3 + 1];
                let blue = self.data[y * row_size + x * 3 + 2];
                Pixel::RGB(red, green, blue)
            }
            ColorType::RGBAlpha() => {
                let red = self.data[y * row_size + x * 4];
                let green = self.data[y * row_size + x * 4 + 1];
                let blue = self.data[y * row_size + x * 4 + 2];
                let alpha = self.data[y * row_size + x * 4 + 3];
                Pixel::RGBAlpha(red, green, blue, alpha)
            }
            ColorType::GrayAlpha() => {
                let gray = self.data[y * row_size + x * 2];
                let alpha = self.data[y * row_size + x * 2 + 1];
                Pixel::GrayAlpha(gray, alpha)
            }
        }
    }

    pub fn flip_vertical(&self) -> Image {
        let mut data = Vec::with_capacity(self.width * self.height);
        let row_size = self.row_size();

        for x in (0..self.height).rev() {
            let start = x * row_size;
            let end = start + row_size;
            let row = &self.data[start..end];
            data.extend_from_slice(row);
        }

        Image {
            height: self.height,
            width: self.width,
            color_type: self.color_type,
            data,
        }
    }

    // This is broken but in a fun way
    // Need to reverse reach row pixel by pixel
    pub fn flip_horizontal(&self) -> Image {
        let mut data = Vec::with_capacity(self.width * self.height);
        let row_size = self.row_size();

        for x in 0..self.height {
            let start = x * row_size;
            let end = start + row_size;
            let mut row = self.data[start..end].to_vec();
            row.reverse();
            data.append(&mut row);
        }

        Image {
            height: self.height,
            width: self.width,
            color_type: self.color_type,
            data,
        }
    }

    pub fn convert(&self, color_type: ColorType) -> Image {
        let mut data = Vec::with_capacity(self.width * self.height);
        let height = self.height;
        let width = self.width;

        for i in 0..height {
            for j in 0..width {
                data.append(&mut self.get_pixel(j, i).convert(color_type).into_vec());
            }
        }

        Image {
            height: self.height,
            width: self.width,
            color_type,
            data,
        }
    }

    fn crop_left(&self, amt: usize) -> Image {
        if self.width <= amt {
            return self.clone();
        }

        let mut data = Vec::with_capacity(self.width * self.height);
        let row_size = self.row_size();
        let num_channels = self.color_type.num_channels() as usize;

        for i in 0..self.height {
            let start = i * row_size;
            let end = start + row_size;
            let row = &self.data[start + num_channels * amt..end];
            data.extend_from_slice(row);
        }

        Image {
            height: self.height,
            width: self.width - amt,
            color_type: self.color_type,
            data,
        }
    }

    fn crop_right(&self, amt: usize) -> Image {
        if self.width <= amt {
            return self.clone();
        }

        let mut data = Vec::with_capacity(self.width * self.height);
        let row_size = self.row_size();
        let num_channels = self.color_type.num_channels() as usize;

        for i in 0..self.height {
            let start = i * row_size;
            let end = start + row_size;
            let row = &self.data[start..end - num_channels * amt];
            data.extend_from_slice(row);
        }

        Image {
            height: self.height,
            width: self.width - amt,
            color_type: self.color_type,
            data,
        }
    }

    fn crop_top(&self, amt: usize) -> Image {
        if self.height <= amt {
            return self.clone();
        }

        let data = self.data[amt * self.row_size()..].to_vec();

        Image {
            height: self.height - amt,
            width: self.width,
            color_type: self.color_type,
            data,
        }
    }

    fn crop_bottom(&self, amt: usize) -> Image {
        if self.height <= amt {
            return self.clone();
        }

        let end = self.height * self.row_size() - amt * self.row_size();
        let data = self.data[..end].to_vec();

        Image {
            height: self.height - amt,
            width: self.width,
            color_type: self.color_type,
            data,
        }
    }

    pub fn crop(&self, left: usize, right: usize, top: usize, bottom: usize) -> Image {
        // do left and right last since they're more resource intensive
        // this way that have to crop less rows
        self.crop_top(top)
            .crop_bottom(bottom)
            .crop_left(left)
            .crop_right(right)
    }

    pub fn write_to_file(mut self, file_name: &str) {
        let png_writer = match PNG::write_png(file_name) {
            Ok(png) => png,
            Err(msg) => {
                println!("Error writing image to file:\n{}", msg);
                return;
            }
        };

        let mut data = Vec::with_capacity(self.height);
        let row_size = self.row_size();

        for x in 0..self.height {
            let start = x * row_size;
            let end = start + row_size;
            let row = &mut self.data[start..end];
            data.push(row.as_mut_ptr());
        }

        let data = data.as_mut_ptr();

        unsafe {
            png_set_IHDR(
                png_writer.png_struct,
                png_writer.png_info,
                self.width as u32,
                self.height as u32,
                8, // This program only supports a bit depth of 8
                self.color_type.as_c_int(),
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_DEFAULT,
                PNG_FILTER_TYPE_DEFAULT,
            );

            png_set_rows(png_writer.png_struct, png_writer.png_info, data);
            png_write_png(
                png_writer.png_struct,
                png_writer.png_info,
                PNG_TRANSFORM_IDENTITY,
                ptr::null_mut(),
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Pixel {
    Gray(u8),
    RGB(u8, u8, u8),
    RGBAlpha(u8, u8, u8, u8),
    GrayAlpha(u8, u8),
}

fn rgb_to_gray(r: u8, g: u8, b: u8) -> u8 {
    let r = 0.3 * (r as f32);
    let g = 0.59 * (g as f32);
    let b = 0.11 * (b as f32);

    (r + g + b) as u8
}

impl Pixel {
    fn into_vec(self) -> Vec<u8> {
        match self {
            Pixel::Gray(g) => vec![g],
            Pixel::RGB(r, g, b) => vec![r, g, b],
            Pixel::RGBAlpha(r, g, b, a) => vec![r, g, b, a],
            Pixel::GrayAlpha(g, a) => vec![g, a],
        }
    }

    fn convert(self, color_type: ColorType) -> Pixel {
        match self {
            Pixel::Gray(g) => match color_type {
                ColorType::Gray() => Pixel::Gray(g),
                ColorType::RGB() => Pixel::RGB(g, g, g),
                ColorType::RGBAlpha() => Pixel::RGBAlpha(g, g, g, u8::max_value()),
                ColorType::GrayAlpha() => Pixel::GrayAlpha(g, u8::max_value()),
            },
            Pixel::RGB(r, g, b) => match color_type {
                ColorType::Gray() => Pixel::Gray(rgb_to_gray(r, g, b)),
                ColorType::RGB() => Pixel::RGB(r, g, b),
                ColorType::RGBAlpha() => Pixel::RGBAlpha(r, g, b, u8::max_value()),
                ColorType::GrayAlpha() => Pixel::GrayAlpha(rgb_to_gray(r, g, b), u8::max_value()),
            },
            Pixel::RGBAlpha(r, g, b, a) => match color_type {
                ColorType::Gray() => Pixel::Gray(rgb_to_gray(r, g, b)),
                ColorType::RGB() => Pixel::RGB(r, g, b),
                ColorType::RGBAlpha() => Pixel::RGBAlpha(r, g, b, a),
                ColorType::GrayAlpha() => Pixel::GrayAlpha(rgb_to_gray(r, g, b), a),
            },
            Pixel::GrayAlpha(g, a) => match color_type {
                ColorType::Gray() => Pixel::Gray(g),
                ColorType::RGB() => Pixel::RGB(g, g, g),
                ColorType::RGBAlpha() => Pixel::RGBAlpha(g, g, g, a),
                ColorType::GrayAlpha() => Pixel::GrayAlpha(g, a),
            },
        }
    }
}

use std::ops::{Index, Range};

use super::bmpg8::BmpImg;
use bevy::{
    prelude::Vec2,
    reflect::{FromReflect, Reflect},
};

#[derive(Clone, Copy, PartialEq, Debug, Reflect, FromReflect)]
pub enum EnvType {
    Meadow,
    Forest,
    Food,
    Water,
    Outside,
}
impl EnvType {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Meadow,
            1 => Self::Forest,
            2 => Self::Food,
            3 => Self::Water,
            _ => panic!("Invalid environment type: {val}"),
        }
    }

    pub fn get_index(&self) -> usize {
        match self {
            Self::Meadow => 0,
            Self::Forest => 1,
            Self::Food => 2,
            Self::Water => 3,
            Self::Outside => 4,
        }
    }
}

#[derive(Debug)]
pub struct Map<T> {
    rows: usize,
    cols: usize,
    data: Vec<T>,
}
impl Map<u8> {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bmp = BmpImg::from_bytes(bytes);
        Self::from(bmp)
    }
    // pub fn size(&self) -> [usize; 2] {
    //     [self.rows, self.cols]
    // }
    /**
     * limits as (min, max)
     */
    pub fn get_index(&self, loc: Vec2, x_lim: (f32, f32), y_lim: (f32, f32)) -> (usize, usize) {
        let x = (loc.x - x_lim.0) / (x_lim.1 - x_lim.0);
        let y = (loc.y - y_lim.0) / (y_lim.1 - y_lim.0);
        let row = (y * self.rows as f32).round() as usize;
        let col = (x * self.cols as f32).round() as usize;
        (row, col)
    }
    pub fn get_env_type(&self, loc: Vec2, x_lim: (f32, f32), y_lim: (f32, f32)) -> EnvType {
        if !(x_lim.0..=x_lim.1).contains(&loc.x) || !(y_lim.0..=y_lim.1).contains(&loc.y) {
            return EnvType::Outside;
        }
        let (row, col) = self.get_index(loc, x_lim, y_lim);
        EnvType::from_u8(self[(row.min(self.rows - 1), col.min(self.cols - 1))])
    }
}
impl Index<Range<usize>> for Map<u8> {
    type Output = [u8];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.data[range]
    }
}
impl Index<(usize, usize)> for Map<u8> {
    type Output = u8;
    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.data[row * self.cols + col]
    }
}
impl From<BmpImg> for Map<u8> {
    fn from(value: BmpImg) -> Self {
        Self {
            rows: value.rows,
            cols: value.cols,
            data: value.data,
        }
    }
}

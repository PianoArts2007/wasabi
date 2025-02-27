mod ram;
mod shared;
use enum_dispatch::enum_dispatch;
use palette::convert::FromColorUnclamped;
use rand::Rng;

pub use ram::{InRamMIDIFile, MIDIFileStats};

use self::shared::timer::TimeKeeper;

/// A struct that represents the view range of a midi screen render
#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIViewRange {
    pub start: f64,
    pub end: f64,
}

impl MIDIViewRange {
    pub fn new(start: f64, end: f64) -> Self {
        MIDIViewRange { start, end }
    }

    pub fn length(&self) -> f64 {
        self.end - self.start
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIColor(u32);

impl MIDIColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        let num = (b as u32) | ((g as u32) << 8) | ((r as u32) << 16);
        MIDIColor(num)
    }

    pub fn new_from_hue(hue: f64) -> Self {
        let hsv = palette::Hsv::new(hue, 1.0, 0.8);
        let rgb = palette::rgb::Rgb::from_color_unclamped(hsv);
        Self::new(
            (rgb.red * 255.0) as u8,
            (rgb.green * 255.0) as u8,
            (rgb.blue * 255.0) as u8,
        )
    }

    pub fn new_vec_for_tracks(tracks: usize) -> Vec<Self> {
        let count = tracks * 16;

        let mut vec = Vec::with_capacity(count);
        for i in 0..count {
            vec.push(MIDIColor::new_from_hue(i as f64 * 360.0 / 16.0 * 15.0));
        }

        vec
    }

    pub fn new_random_vec_for_tracks(tracks: usize) -> Vec<Self> {
        let count = tracks * 16;

        let mut vec = Vec::with_capacity(count);
        for _ in 0..count {
            let r = rand::thread_rng().gen_range(0..255) as u8;
            let g = rand::thread_rng().gen_range(0..255) as u8;
            let b = rand::thread_rng().gen_range(0..255) as u8;
            vec.push(MIDIColor::new(r, g, b));
        }

        vec
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn red(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn green(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn blue(&self) -> u8 {
        (self.0 >> 0) as u8
    }
}

/// The basic shared functions in a midi file. The columns related functions are
/// inside the [`MIDIFile`] trait.
#[enum_dispatch]
pub trait MIDIFileBase {
    fn midi_length(&self) -> Option<f64>;
    fn parsed_up_to(&self) -> Option<f64>;

    fn timer(&self) -> &TimeKeeper;
    fn timer_mut(&mut self) -> &mut TimeKeeper;

    fn stats(&self) -> MIDIFileStats;

    fn allows_seeking_backward(&self) -> bool;
}

/// This trait contains a function to retrieve the column view of the midi
pub trait MIDIFile: MIDIFileBase {
    type ColumnsViews<'a>: 'a + MIDINoteViews
    where
        Self: 'a;

    fn get_current_column_views<'a>(&'a mut self, range: &mut f64) -> Self::ColumnsViews<'a>;
}

#[enum_dispatch]
pub trait MIDINoteViewsBase {}

pub trait MIDINoteViews {
    type View<'a>: 'a + MIDINoteColumnView
    where
        Self: 'a;

    fn get_column<'a>(&'a self, key: usize) -> Self::View<'a>;
    fn range<'a>(&'a self) -> MIDIViewRange;
}

pub trait MIDINoteColumnView: Send {
    type Iter<'a>: 'a + ExactSizeIterator<Item = DisplacedMIDINote> + Send
    where
        Self: 'a;

    fn iterate_displaced_notes<'a>(&'a self) -> Self::Iter<'a>;
}

pub struct DisplacedMIDINote {
    pub start: f32,
    pub len: f32,
    pub color: MIDIColor,
}

#[enum_dispatch(MIDIFileBase)]
pub enum MIDIFileUnion {
    InRam(ram::InRamMIDIFile),
}

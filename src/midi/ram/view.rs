use std::{ops::Range, sync::Arc};

use gen_iter::GenIter;

use crate::midi::{
    DisplacedMIDINote, MIDIColor, MIDINoteColumnView, MIDINoteViews, MIDINoteViewsBase,
    MIDIViewRange,
};

use super::column::InRamNoteColumn;

pub struct InRamNoteViews {
    columns: Arc<Vec<InRamNoteColumn>>,
    column_view_data: Vec<InRamNoteColumnViewData>,
    default_track_colors: Vec<MIDIColor>,
    view_range: MIDIViewRange,
}

impl InRamNoteViews {
    pub fn new(columns: Arc<Vec<InRamNoteColumn>>, track_count: usize) -> Self {
        let column_view_data = columns
            .iter()
            .map(|_| InRamNoteColumnViewData::new())
            .collect();
        InRamNoteViews {
            columns,
            column_view_data,
            view_range: MIDIViewRange {
                start: 0.0,
                end: 0.0,
            },
            default_track_colors: MIDIColor::new_vec_for_tracks(track_count),
        }
    }
}

pub struct InRamNoteColumnViewData {
    notes_to_end: usize,
    notes_to_start: usize,
    block_range: Range<usize>,
}

impl InRamNoteColumnViewData {
    pub fn new() -> Self {
        InRamNoteColumnViewData {
            notes_to_end: 0,
            notes_to_start: 0,
            block_range: 0..0,
        }
    }
}

pub struct InRamNoteColumnView<'a> {
    view: &'a InRamNoteViews,
    column: &'a InRamNoteColumn,
    data: &'a InRamNoteColumnViewData,
    view_range: MIDIViewRange,
}

impl MIDINoteViewsBase for InRamNoteViews {
    fn shift_view_range(&mut self, new_view_range: MIDIViewRange) {
        let old_view_range = self.view_range;
        self.view_range = new_view_range;

        for (column, data) in self.columns.iter().zip(self.column_view_data.iter_mut()) {
            if column.blocks.len() == 0 {
                continue;
            }

            let mut new_block_start = data.block_range.start;
            let mut new_block_end = data.block_range.end;

            if new_view_range.end > old_view_range.end {
                while new_block_end < column.blocks.len() {
                    let block = &column.blocks[new_block_end];
                    if block.start >= new_view_range.end {
                        break;
                    }
                    data.notes_to_end += block.notes.len();
                    new_block_end += 1;
                }
            } else if new_view_range.end < old_view_range.end {
                while new_block_end > 0 {
                    let block = &column.blocks[new_block_end - 1];
                    if block.start < new_view_range.end {
                        break;
                    }
                    data.notes_to_end -= block.notes.len();
                    new_block_end -= 1;
                }
            } else {
                // No change in view end
            }

            if new_view_range.start > old_view_range.start {
                while new_block_start < column.blocks.len() {
                    let block = &column.blocks[new_block_start];
                    if block.max_end() >= new_view_range.start {
                        break;
                    }
                    data.notes_to_start += block.notes.len();
                    new_block_start += 1;
                }
            } else if new_view_range.start < old_view_range.start {
                // It is smaller, we have to start from the beginning
                data.notes_to_start = 0;
                new_block_start = 0;
                while new_block_start < column.blocks.len() {
                    let block = &column.blocks[new_block_start];
                    if block.max_end() >= new_view_range.start {
                        break;
                    }
                    data.notes_to_start += block.notes.len();
                    new_block_start += 1;
                }
            } else {
                // No change in view start
            }

            data.block_range = new_block_start..new_block_end;
        }
    }

    fn allows_seeking_backward(&self) -> bool {
        false
    }
}

impl MIDINoteViews for InRamNoteViews {
    type View<'a> = InRamNoteColumnView<'a>;

    fn get_column<'a>(&'a self, key: usize) -> Self::View<'a> {
        InRamNoteColumnView {
            view: self,
            column: &self.columns[key],
            data: &self.column_view_data[key],
            view_range: self.view_range,
        }
    }

    fn range<'a>(&'a self) -> MIDIViewRange {
        self.view_range
    }
}

impl MIDINoteViews for &InRamNoteViews {
    type View<'a> = InRamNoteColumnView<'a> where Self: 'a;

    fn get_column<'a>(&'a self, key: usize) -> Self::View<'a> {
        InRamNoteColumnView {
            view: self,
            column: &self.columns[key],
            data: &self.column_view_data[key],
            view_range: self.view_range,
        }
    }

    fn range<'a>(&'a self) -> MIDIViewRange {
        self.view_range
    }
}

impl<'a> InRamNoteColumnView<'a> {}

struct InRamNoteBlockIter<'a, Iter: Iterator<Item = DisplacedMIDINote>> {
    view: &'a InRamNoteColumnView<'a>,
    iter: Iter,
}

impl<'a> MIDINoteColumnView for InRamNoteColumnView<'a> {
    type Iter<'b> = impl 'b + ExactSizeIterator<Item = DisplacedMIDINote> where Self: 'b;

    fn iterate_displaced_notes<'b>(&'b self) -> Self::Iter<'b> {
        let colors = &self.view.default_track_colors;

        let iter = GenIter(move || {
            for block_index in self.data.block_range.clone().rev() {
                let block = &self.column.blocks[block_index];
                let start = (block.start - self.view_range.start) as f32;

                for note in block.notes.iter().rev() {
                    yield DisplacedMIDINote {
                        start: start,
                        len: note.len,
                        color: colors[note.track_chan as usize].as_u32(),
                    };
                }
            }
        });

        InRamNoteBlockIter {
            view: self,
            iter: iter.into_iter(),
        }
    }

    fn adjust_view_range(&mut self, range: MIDIViewRange) {
        self.view_range = range;
    }
}

impl<Iter: Iterator<Item = DisplacedMIDINote>> Iterator for InRamNoteBlockIter<'_, Iter> {
    type Item = DisplacedMIDINote;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<Iter: Iterator<Item = DisplacedMIDINote>> ExactSizeIterator for InRamNoteBlockIter<'_, Iter> {
    fn len(&self) -> usize {
        if self.view.data.notes_to_end < self.view.data.notes_to_start {
            dbg!(
                self.view.data.notes_to_end,
                self.view.data.notes_to_start,
                &self.view.data.block_range
            );
        }
        self.view.data.notes_to_end - self.view.data.notes_to_start
    }
}

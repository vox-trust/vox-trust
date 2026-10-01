//! The public chip pattern: which tiles of a window form which group, the sign of each tile,
//! and each group's dither (and known bit, for synchronisation groups).

use crate::{fec, Params};

/// SplitMix64: a small, well-known generator. The pattern is public, so it only needs to be
/// reproducible and well mixed, not secret.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// A group of chips quantised together.
pub(crate) struct Group {
    /// Tiles, numbered `column * subbands + subband` within a window.
    pub(crate) chips: Vec<usize>,
    /// Lattice offset, as a fraction of the step.
    pub(crate) dither: f32,
    /// For synchronisation groups: the bit they always carry.
    pub(crate) known_bit: bool,
}

pub(crate) struct Layout {
    signs: Vec<f32>,
    data: Vec<Group>,
    sync: Vec<Group>,
}

impl Layout {
    pub(crate) fn new(p: &Params) -> Layout {
        let tiles = p.columns * p.subbands();
        let mut rng = SplitMix64(p.pattern_seed);
        let signs = (0..tiles)
            .map(|_| if rng.next() & 1 == 1 { 1.0 } else { -1.0 })
            .collect();
        // Fisher-Yates shuffle, so every group's chips are spread over the whole window.
        let mut order: Vec<usize> = (0..tiles).collect();
        for i in (1..tiles).rev() {
            order.swap(i, rng.below(i + 1));
        }
        let n_sync = tiles / p.sync_every;
        let (sync_tiles, data_tiles) = order.split_at(n_sync);
        let per_group = data_tiles.len() / fec::CODED_BITS;
        let mut make = |tiles: &[usize], count: usize| -> Vec<Group> {
            let mut groups: Vec<Group> = (0..count)
                .map(|_| Group {
                    chips: Vec::with_capacity(per_group),
                    dither: rng.unit(),
                    known_bit: rng.next() & 1 == 1,
                })
                .collect();
            if count > 0 {
                for (k, &t) in tiles.iter().take(per_group * count).enumerate() {
                    groups[k % count].chips.push(t);
                }
            }
            groups
        };
        let data = make(data_tiles, fec::CODED_BITS);
        let sync_count = sync_tiles.len().checked_div(per_group).unwrap_or(0);
        let sync = make(sync_tiles, sync_count);
        Layout { signs, data, sync }
    }

    pub(crate) fn sign(&self, tile: usize) -> f32 {
        self.signs[tile]
    }

    pub(crate) fn data_groups(&self) -> &[Group] {
        &self.data
    }

    pub(crate) fn sync_groups(&self) -> &[Group] {
        &self.sync
    }

    pub(crate) fn chips_per_group(&self) -> usize {
        self.data.first().map_or(0, |g| g.chips.len())
    }
}

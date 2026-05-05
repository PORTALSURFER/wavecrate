mod common;
mod destructive;
mod metadata;
mod transforms;

use super::*;
use crate::app::state::LoopCrossfadeSettings;

pub(crate) trait BrowserActions {
    fn tag_browser_sample(
        &mut self,
        row: usize,
        tag: crate::sample_sources::Rating,
    ) -> Result<(), String>;
    fn tag_browser_samples(
        &mut self,
        rows: &[usize],
        tag: crate::sample_sources::Rating,
        primary_visible_row: usize,
    ) -> Result<(), String>;
    fn set_loop_marker_browser_samples(
        &mut self,
        rows: &[usize],
        looped: bool,
        primary_visible_row: usize,
    ) -> Result<(), String>;
    fn set_bpm_browser_samples(
        &mut self,
        rows: &[usize],
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String>;
    fn normalize_browser_sample(&mut self, row: usize) -> Result<(), String>;
    fn normalize_browser_samples(&mut self, rows: &[usize]) -> Result<(), String>;
    fn loop_crossfade_browser_samples(
        &mut self,
        rows: &[usize],
        settings: LoopCrossfadeSettings,
        primary_visible_row: usize,
    ) -> Result<(), String>;
    fn rename_browser_sample(&mut self, row: usize, new_name: &str) -> Result<(), String>;
    fn delete_browser_sample(&mut self, row: usize) -> Result<(), String>;
    fn delete_browser_samples(&mut self, rows: &[usize]) -> Result<(), String>;
}

impl BrowserActions for BrowserController<'_> {
    fn tag_browser_sample(
        &mut self,
        row: usize,
        tag: crate::sample_sources::Rating,
    ) -> Result<(), String> {
        self.tag_browser_sample_action(row, tag)
    }

    fn tag_browser_samples(
        &mut self,
        rows: &[usize],
        tag: crate::sample_sources::Rating,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        self.tag_browser_samples_action(rows, tag, primary_visible_row)
    }

    fn set_loop_marker_browser_samples(
        &mut self,
        rows: &[usize],
        looped: bool,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        self.set_loop_marker_browser_samples_action(rows, looped, primary_visible_row)
    }

    fn set_bpm_browser_samples(
        &mut self,
        rows: &[usize],
        bpm: f32,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        self.set_bpm_browser_samples_action(rows, bpm, primary_visible_row)
    }

    fn normalize_browser_sample(&mut self, row: usize) -> Result<(), String> {
        self.normalize_browser_sample_action(row)
    }

    fn normalize_browser_samples(&mut self, rows: &[usize]) -> Result<(), String> {
        self.normalize_browser_samples_action(rows)
    }

    fn loop_crossfade_browser_samples(
        &mut self,
        rows: &[usize],
        settings: LoopCrossfadeSettings,
        primary_visible_row: usize,
    ) -> Result<(), String> {
        self.loop_crossfade_browser_samples_action(rows, settings, primary_visible_row)
    }

    fn rename_browser_sample(&mut self, row: usize, new_name: &str) -> Result<(), String> {
        self.rename_browser_sample_action(row, new_name)
    }

    fn delete_browser_sample(&mut self, row: usize) -> Result<(), String> {
        self.delete_browser_sample_action(row)
    }

    fn delete_browser_samples(&mut self, rows: &[usize]) -> Result<(), String> {
        self.delete_browser_samples_action(rows)
    }
}

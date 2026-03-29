use crate::app::state::{DestructiveEditPrompt, DestructiveSelectionEdit};

impl DestructiveSelectionEdit {
    fn title(&self) -> &'static str {
        match self {
            DestructiveSelectionEdit::CropSelection => "Crop selection",
            DestructiveSelectionEdit::TrimSelection => "Trim selection",
            DestructiveSelectionEdit::ReverseSelection => "Reverse selection",
            DestructiveSelectionEdit::FadeLeftToRight => "Fade selection (left to right)",
            DestructiveSelectionEdit::FadeRightToLeft => "Fade selection (right to left)",
            DestructiveSelectionEdit::ShortEdgeFades => "Add short edge fades",
            DestructiveSelectionEdit::MuteSelection => "Mute selection",
            DestructiveSelectionEdit::NormalizeSelection => "Normalize selection",
            DestructiveSelectionEdit::ClickRemoval => "Remove clicks in selection",
            DestructiveSelectionEdit::CleanExactDuplicateBeats => "Clean duplicate windows",
        }
    }

    fn warning(&self) -> &'static str {
        match self {
            DestructiveSelectionEdit::CropSelection => {
                "This will overwrite the file with only the selected region."
            }
            DestructiveSelectionEdit::TrimSelection => {
                "This will remove the selected region and close the gap in the source file."
            }
            DestructiveSelectionEdit::ReverseSelection => {
                "This will overwrite the selection with the audio reversed in time."
            }
            DestructiveSelectionEdit::FadeLeftToRight => {
                "This will overwrite the selection with a fade down to silence."
            }
            DestructiveSelectionEdit::FadeRightToLeft => {
                "This will overwrite the selection with a fade up from silence."
            }
            DestructiveSelectionEdit::ShortEdgeFades => {
                "This will overwrite the selection with short fade-ins and fade-outs."
            }
            DestructiveSelectionEdit::MuteSelection => {
                "This will overwrite the selection with silence."
            }
            DestructiveSelectionEdit::NormalizeSelection => {
                "This will overwrite the selection with a normalized version and short fades."
            }
            DestructiveSelectionEdit::ClickRemoval => {
                "This will overwrite the selection with an interpolated repair to remove clicks."
            }
            DestructiveSelectionEdit::CleanExactDuplicateBeats => {
                "This will remove later duplicate windows and close the gaps in the source file."
            }
        }
    }
}

pub(crate) fn prompt_for_edit(edit: DestructiveSelectionEdit) -> DestructiveEditPrompt {
    DestructiveEditPrompt {
        edit,
        title: edit.title().to_string(),
        message: edit.warning().to_string(),
    }
}

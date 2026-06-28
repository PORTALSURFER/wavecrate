# Metadata, Tags, and Collections

Wavecrate supports lightweight metadata so a fast audition pass can leave useful traces without requiring a permanent folder move.

## Ratings

Ratings are a quick keep-or-trash signal for sample rows.

- `]` raises the selected sample rating.
- `[` lowers the selected sample rating.
- Rating filters in the sidebar narrow the visible list.

Use ratings when you want a fast pass through a folder before deciding whether samples deserve tags, collections, or edits.

## Tags

Tags are durable labels for finding sounds again.

- Use the tag input in the sidebar to add tags to the selected sample.
- Type in the tag field to use autocomplete.
- `Escape` cancels tag entry or tag completion.
- `Up` and `Down` move through tag completion suggestions.
- `Delete` or `Backspace` deletes the selected tag token.
- `9` toggles the `one-shot` tag on selected samples.
- `0` toggles the `loop` tag on selected samples.

Right-click a tag token to delete it from the selected sample.

## Name and Tag Filters

The sidebar includes text filters for sample names and tags.

- **Name:** filters rows by fuzzy filename or display-name text.
- **Tags:** filters rows by tag text.

These filters combine with folders, ratings, collections, playback type, Harvest filters, similarity search, and Starmap.

## Collections

Collections are temporary color buckets for a working pass. They let you gather candidates without moving the original files.

- `1` through `6` toggles selected samples in the matching collection.
- Click a collection to focus it.
- Press `Escape` while collection focus is active to return to normal browsing.
- Drag selected samples onto a collection to add them.
- Right-click a collection to clear broken files from that collection.

Collection names can be renamed in the collection panel. Collection membership is metadata; it does not move the sample on disk.

## Disk Names and Labels

The sample browser can switch between disk filenames and metadata labels.

- **Disk** shows the file name from disk.
- **Label** shows the metadata label when one is available.

Use disk names when checking actual file organization. Use labels when you want the browser to reflect the musical or production name you assigned inside Wavecrate.

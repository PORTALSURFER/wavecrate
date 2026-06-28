# Sources, Locks, and Harvest

Sources are the roots Wavecrate indexes. A source can be an ordinary sample folder, a protected project folder, or a folder that participates in the Harvest workflow.

## Ordinary Sources

An ordinary source is a local folder you allow Wavecrate to index and work with directly. Wavecrate can read files, track metadata, and perform file operations you explicitly request.

Use ordinary sources for:

- dedicated sample folders
- exported one-shots
- folders you already back up
- material you are comfortable editing in place

## Protected and Locked Sources

Protected sources are for original project material, long recordings, Ableton folders, field recordings, or anything where you do not want Wavecrate to casually mutate the original.

Protected behavior keeps the workflow creative while reducing risk:

- Original material stays in place.
- Derived clips are written into a writable destination.
- Harvest metadata records the relationship between the original and the derived file.
- Destructive-style actions are routed toward copies where the source role requires it.

Locked folders are stricter. Use locking when a source should be visible and auditionable but not mutated through normal edit commands.

## Harvest Concepts

Harvest is Wavecrate's way of tracking how raw material turns into reusable samples.

- **Origin:** the original file or recording.
- **Derivative:** a new file created from an origin, usually by extraction or a protected-source edit.
- **Touched:** the origin has been reviewed or used.
- **Extracted:** useful material has been pulled from the origin.
- **Done or ignored:** the origin no longer needs review in the current workflow.

Harvest is metadata-driven. It is not meant to replace your folder structure or force planning into markdown files.

## Recommended Source Strategy

- Use ordinary sources for disposable or already-curated sample libraries.
- Use protected sources for projects, long recordings, and source material you may want to revisit.
- Keep one clear writable destination for derived clips.
- Back up important folders before using destructive actions.

# Similarity and Starmap

Similarity tools help you move by sound instead of only by folder name, filename, or tag.

## Automatic Similarity Processing

Similarity search and Starmap depend on source processing, which reconciles automatically after source changes. Use **Process Source** only to prioritize that source.

Large sources can take time to process. Wavecrate keeps browsing responsive while processing work runs in the background.

## Similarity Search

Similarity search uses the selected sample as the reference sound.

- Wait for automatic source processing when the source has just changed.
- Select a sample to use as the reference.
- Use similarity search to focus the list around related sounds.
- Combine similarity with folder scope, search text, tags, ratings, collections, playback type, and Harvest filters.

When similarity results are active, the sample browser can show similarity scores and aspect controls.

## Similarity Aspects

The browser header exposes similarity aspect controls when similarity data is available.

- **Overall:** balanced similarity.
- **Spectrum:** broad frequency-shape similarity.
- **Timbre:** tone-color similarity.
- **Pitch:** pitch or tonal-center similarity where available.
- **Amp:** amplitude and envelope similarity.
- **Weight:** toggles weighted aspect behavior.

Use aspects to steer the search. For example, turn toward spectrum or timbre when looking for a similar color, or amplitude when envelope shape matters more.

## Starmap

Starmap shows the current scoped result set as a spatial similarity view.

- The Starmap button switches between row list and map view.
- The search field filters the same scoped results shown in the map.
- Zoom controls change map scale.
- The target button focuses the selected node.
- The reset button returns the map viewport to its default framing.
- `F` focuses the selected Starmap node.

Each point represents a listed sample. Nearby points are more related than distant points within the current Starmap layout.

## Starmap Auditioning

Use Starmap when you want to scan related sounds as clusters.

- Click a point to select and audition the sample.
- Drag through points to audition nearby sounds quickly.
- Use the list toggle to return to rows without losing source, folder, search, or filter context.

The status line shows how many current results have Starmap positions. If the count is lower than the listed total, process the source and wait for analysis to finish.

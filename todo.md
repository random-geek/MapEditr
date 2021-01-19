Optimizations:

- Cache deserialized map blocks instead of deserializing each time.
- Overlay: Iterate map blocks in space-filling curve shape to optimize cache usefulness.
- (DONE) Don't re-compress node data/metadata if it isn't changed.
- MapBlock::serialize: Use a big buffer somewhere to avoid heap allocations.

Todo?

- Fold area utility functions into the area struct.
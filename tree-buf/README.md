# TreeBuf

TreeBuf is a binary serialization format akin to ProtoBuf and FlatBuffers.

It's most suited for medium to large data sets that generally conform to a schema which may change.

The design is currently in prototype stage and the file format unstable. So, don't use this if you need to persist versioned data but you can experiment and give feedback.

## Goals / Features:
 * Fast serialization and deserialization
 * Small wire size
 * Self-describing
 * Flexible
 * Backwards compatibility
 * Forwards compatibility
 
 The ultimate goal of TreeBuf is to be on the convex hull of the Pareto Front minimizing serialization time, deserialization time, and wire size for real world data when compared to other protocols with similar feature sets.
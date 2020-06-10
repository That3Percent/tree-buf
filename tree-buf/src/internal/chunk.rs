// Chunks require:
// Size in bytes (dependent on compression scheme)
// Count of items in the chunk, regardless of compression scheme
//   The number of items can be used to skip chunks.

/*
Consider data:

[
    {
        a: [a0, a1, a2, a3],
        b: {
            x: x0,
            y: y0
        },
        c: Null,
    },
    {
        a: [a4, a5, a6, a7],
        b: {
            x: x1,
            y: y1
        },
        c: Null
    }
}

// Needs:
// Writing flushes in chunks to the compressor, which may or may not already be initialized/chosen
// A compressor may return Err(remainder) when _some_ are compressed, which may cause the buffer to "shuffle down" and re-compress
// Whether the compressor encodes _any_
// When specify "Chunked" each chunk gets a fixed amount of elements, except the last

Window size = 2

ALTERNATE: Any chunk can refer to any previous parent chunk.
So, when we create Array and give it a parent, we are creating a _commitment_ to include
the length buffer at some later point in time.
Difficulty - "skipping" through the file for random access

// When writing - buffer n items then flush chunks
// When flushing the first chunk, ensure the parent "Object"
// has been written and in the same method retrieve it's id
// so that the chunk can refer to the parent.

Start writing:
    Array, len 2
       Obj props 2 <- reserve len but don't encode? Instead of count of props, use null-terminiated prop?
         a: Array (unknown if fixed or variable at this point) {
             start buffering len: [4]
             start buffering a: [a0, a1, a2] chunk filled, flush
             Chunk: [0, 1, 2] assigned continuation id 0
             continue buffering a: [a3]
             Now what? the len property has no output
         }
         Now what? Is this a continued or is it b?


buffer(value) & buffer(values) APIs can avoid copies in some cases

Only start writing prop id when flushing for the first time
After sampling data, flush batches of data until byte buffer full, then copy to output.
When flushing if something fails to encode using an encoding have the failure return the remainder of the slice
It is ok if the encoding changes mid-stream
*/

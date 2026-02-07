# adabraka-gpui Performance Optimization Plan

Comprehensive audit of the GPUI rendering pipeline, memory allocation patterns, layout engine, and GPU layer. Findings prioritized by impact and implementation complexity.

---

## Tier 1: Quick Wins (High Impact, Low Effort)

### 1.1 Eliminate Double-Storage of Primitives in Scene
**File**: `crates/gpui/src/scene.rs:67-115`
**Problem**: Every primitive is stored TWICE - once in `paint_operations: Vec<PaintOperation>` (as `Primitive` enum) and again in the type-specific vec (`shadows`, `quads`, etc.). Each `insert_primitive()` call clones the primitive:
```rust
Primitive::Shadow(shadow) => {
    shadow.order = order;
    self.shadows.push(shadow.clone()); // Clone #1
}
// ...
self.paint_operations.push(PaintOperation::Primitive(primitive)); // Clone #2 (move)
```
**Fix**: Store only in type-specific vecs. Replace `paint_operations` with a lightweight `Vec<(PrimitiveKind, usize)>` that stores just the kind and index. The `replay()` method can reconstruct from indices.
**Impact**: HIGH - Eliminates 1 clone per primitive per frame. For a complex UI with 5000+ primitives, this saves ~200KB+ of allocation/copy per frame.

### 1.2 Pre-allocate Scene Vectors with Previous Frame Capacity
**File**: `crates/gpui/src/scene.rs:38-49`
**Problem**: `Scene::clear()` calls `.clear()` on all vectors, which preserves capacity. But `Scene::default()` creates empty vectors with zero capacity, meaning the first few frames cause many reallocations.
**Fix**: Add `Scene::with_capacity()` or after swap, use `std::mem::swap` to reuse the previous scene's allocated buffers:
```rust
pub fn clear_reuse(&mut self, prev: &mut Scene) {
    std::mem::swap(&mut self.shadows, &mut prev.shadows);
    self.shadows.clear();
    // ... etc
}
```
**Impact**: MEDIUM - Eliminates vector growth allocations after first frame. Biggest impact on app startup.

### 1.3 Remove Unnecessary Sorting in Scene::finish()
**File**: `crates/gpui/src/scene.rs:127-137`
**Problem**: All 7 primitive vectors are sorted by `order` every frame:
```rust
self.shadows.sort_by_key(|shadow| shadow.order);
self.quads.sort_by_key(|quad| quad.order);
// ... 5 more sorts
```
Since primitives are mostly inserted in order (element tree traversal is depth-first), these vectors are nearly sorted.
**Fix**: Use `sort_unstable_by_key()` instead of `sort_by_key()` - avoids allocation of temporary storage. Better yet, check if already sorted first with `is_sorted_by_key()` (Rust nightly) or a manual check.
**Impact**: MEDIUM - `sort_unstable_by_key` is faster and doesn't allocate. For already-sorted data, skip entirely.

### 1.4 Avoid Cloning Focus/Dispatch Listeners
**File**: `crates/gpui/src/window.rs:1951-1970`
**Problem**: Focus listeners are cloned every frame during focus path comparison:
```rust
self.focus_lost_listeners.clone().retain(&(), |listener| listener(self, cx));
self.focus_listeners.clone().retain(&(), |listener| listener(&event, self, cx));
```
**Fix**: Iterate without cloning by taking ownership temporarily or using an index-based approach.
**Impact**: MEDIUM - Avoids cloning the entire listener collection on every frame with focus changes.

### 1.5 Use `SmallVec` for Hot-Path Stacks in Window
**File**: `crates/gpui/src/window.rs:837-842`
**Problem**: Several per-frame stacks use `Vec`:
```rust
pub(crate) text_style_stack: Vec<TextStyleRefinement>,  // TextStyleRefinement is large!
pub(crate) element_offset_stack: Vec<Point<Pixels>>,
pub(crate) content_mask_stack: Vec<ContentMask<Pixels>>,
```
These are push/pop stacks that rarely exceed 8-16 entries (matching typical DOM depth).
**Fix**: Use `SmallVec<[T; 8]>` or `SmallVec<[T; 16]>` for these stacks. `element_id_stack` already uses `SmallVec<[ElementId; 32]>`.
**Impact**: MEDIUM - Eliminates heap allocation for typical UI trees. `element_offset_stack` and `content_mask_stack` are lightweight (32 bytes each), good candidates for stack allocation.

---

## Tier 2: Medium Effort (High Impact, Moderate Changes)

### 2.1 Incremental Scene Building with Dirty View Tracking
**File**: `crates/gpui/src/window.rs:1914-1981`
**Problem**: The system already tracks dirty views (`dirty_views: FxHashSet<EntityId>`) but the `draw()` method rebuilds the entire element tree and scene every frame. Even unchanged views are fully re-rendered.
**Fix**: Implement scene caching per view. When a view is not dirty:
1. Skip its `render()` call
2. Replay its previous paint operations into the new scene via `scene.replay(range, prev_scene)`
The infrastructure for this exists (`Scene::replay` at line 117) but isn't used for view-level caching.
**Impact**: HIGH - For typical UIs where only 1-2 views change per frame, this could skip 80-90% of rendering work. This is the single biggest optimization possible.

### 2.2 Pool and Reuse Frame Allocations
**File**: `crates/gpui/src/window.rs:667-746`
**Problem**: The `Frame` struct contains ~15 vectors that are cleared every frame. While `clear()` preserves capacity, the frame swap at line 1942 (`mem::swap(&mut self.rendered_frame, &mut self.next_frame)`) followed by `self.next_frame.clear()` means the new next_frame reuses the rendered_frame's buffers. This is good. BUT element_states HashMap is rebuilt via remove_entry/insert in `finish()`, which causes hash table churn.
**Fix**: Instead of moving element states one-by-one in `Frame::finish()`, swap the entire HashMap and mark which keys were accessed, lazily evicting stale entries.
**Impact**: MEDIUM - Reduces HashMap overhead for complex UIs with many element states.

### 2.3 Reduce Style Struct Size
**File**: `crates/gpui/src/style.rs:144-284`
**Problem**: The `Style` struct has ~40 fields including nested structs. The Refineable derive generates a `StyleRefinement` struct where most fields are `Option<T>`, making it even larger. Key bloat:
- `box_shadow: Vec<BoxShadow>` - heap allocation even when empty (24 bytes for Vec header)
- `text: TextStyleRefinement` - very large, contains font family SharedString, etc.
- Multiple `Option<f32>`, `Option<u16>` fields waste padding bytes
**Fix**:
- Replace `Vec<BoxShadow>` with `SmallVec<[BoxShadow; 1]>` (most elements have 0-1 shadows)
- Pack boolean fields into a bitfield (`display`, `visibility`, `position`, `flex_direction`, `flex_wrap`, `border_style`, `continuous_corners`, `restrict_scroll_to_axis`, `allow_concurrent_scroll` - 9 fields that could be a single u16)
- Consider splitting Style into "hot" (layout) and "cold" (visual) parts for cache-line friendliness
**Impact**: MEDIUM-HIGH - Style is instantiated for every element. Reducing its size by 30-40% improves cache hit rates and reduces allocation pressure.

### 2.4 Batch Path Rendering Without Command Encoder Recreation
**File**: `crates/gpui/src/platform/mac/metal_renderer.rs:457-459`
**Problem**: Path rendering breaks the command encoder:
```rust
PrimitiveBatch::Paths(paths) => {
    command_encoder.end_encoding();  // Expensive!
    // Creates new encoder for MSAA path rendering
}
```
Every path batch ends the current encoder and creates a new one. If paths are interleaved with quads (which they often are due to draw ordering), this causes many encoder switches.
**Fix**: Batch all paths into a single contiguous draw. Consider separating the path rendering to a pre-pass: render all paths to a texture first, then composite. Or sort paths to minimize encoder switches.
**Impact**: MEDIUM-HIGH - Encoder creation/destruction is expensive on Metal. Minimizing switches improves GPU throughput.

### 2.5 Instance Buffer Sizing and Reuse
**File**: `crates/gpui/src/platform/mac/metal_renderer.rs:370-416`
**Problem**: If the instance buffer is too small, the renderer enters a retry loop that doubles the buffer and re-renders the entire scene:
```rust
loop {
    let mut instance_buffer = self.instance_buffer_pool.lock().acquire(&self.device);
    let command_buffer = self.draw_primitives(scene, &mut instance_buffer, drawable, viewport_size);
    match command_buffer {
        Err(err) => {
            instance_buffer_pool.reset(buffer_size * 2);  // Double and retry everything!
        }
    }
}
```
**Fix**: Track the required instance buffer size from the previous frame and pre-allocate with a 20% margin. Also, pre-calculate the total instance count before encoding to avoid retry.
**Impact**: MEDIUM - Eliminates stalls when buffer is too small. In practice, only affects frames where element count increases significantly.

---

## Tier 3: Architectural Improvements (Transformative Impact, Larger Effort)

### 3.1 Element Tree Caching / Virtual DOM Diffing
**File**: `crates/gpui/src/element.rs` (architecture-level)
**Problem**: From the module docs: "Before the start of the next frame, the entire element tree and any callbacks they have registered with GPUI are dropped and the process repeats." The entire element tree is rebuilt from scratch every frame, even when nothing changed.
**Fix**: Implement element tree memoization/caching:
1. Cache the element tree per view
2. Only rebuild subtrees when their view is dirty
3. Cache layout results (LayoutId -> Bounds) across frames for unchanged elements
This is a significant architectural change but would be transformative.
**Impact**: VERY HIGH - Could reduce per-frame work by 10x for idle UIs. Most frames in a desktop app have zero or minimal changes.

### 3.2 Arena Allocator for Per-Frame Objects
**File**: `crates/gpui/src/arena.rs`, `crates/gpui/src/element.rs`
**Problem**: GPUI already has an `ELEMENT_ARENA` but per-frame objects like mouse listeners (`Vec<Option<AnyMouseListener>>`), hitboxes (`Vec<Hitbox>`), deferred draws, cursor styles etc. use standard heap allocation.
**Fix**: Extend the arena allocator to cover all per-frame allocations. Since everything is dropped at frame end, a bump allocator is perfect:
1. Allocate all per-frame objects from a single arena
2. Reset the arena at frame boundary (O(1) "free")
3. No individual deallocation needed
**Impact**: HIGH - Eliminates thousands of individual alloc/dealloc calls per frame. Bump allocation is 10-100x faster than general-purpose allocation.

### 3.3 GPU-Side Culling and Frustum Clipping
**File**: `crates/gpui/src/scene.rs:67-75`
**Problem**: Clipping is done CPU-side per primitive:
```rust
let clipped_bounds = primitive.bounds().intersect(&primitive.content_mask().bounds);
if clipped_bounds.is_empty() { return; }
```
This is O(N) CPU work for all primitives, even those completely off-screen.
**Fix**:
1. Implement spatial subdivision (quadtree or grid) for the scene
2. Cull off-screen primitives at the batch level rather than individually
3. Move content mask clipping to the GPU shader (pass content_mask as a uniform and discard in fragment shader - this is already partially done)
**Impact**: MEDIUM - Most impactful for large scrollable content where many elements are off-screen.

### 3.4 Parallel Layout + Paint Phases
**File**: `crates/gpui/src/window.rs:1914-1981`
**Problem**: The draw pipeline is sequential: prepaint -> layout -> paint -> finish. For multi-window apps, each window is drawn sequentially.
**Fix**:
1. Run layout on background thread (Taffy is thread-safe)
2. Paint can overlap with GPU command encoding of the previous frame (double-buffer the scene)
3. For multi-window, draw windows in parallel
**Impact**: MEDIUM-HIGH - Utilizes multi-core CPUs. Biggest win for apps with multiple windows or complex layouts.

### 3.5 Text Shaping Cache Improvements
**File**: `crates/gpui/src/text_system/`
**Problem**: Text shaping (converting text + font -> positioned glyphs) is one of the most expensive operations. The current cache is per-window (`WindowTextSystem`) and uses a `LineLayoutCache` that's frame-indexed.
**Fix**:
1. Share text shaping cache across windows (many UI strings are identical)
2. Use content-addressed caching (hash of text + style -> shaped result)
3. Cache at the paragraph level, not just line level
4. Pre-shape common strings (button labels, menu items) at init time
**Impact**: MEDIUM - Text shaping is expensive. Better caching reduces work for text-heavy UIs.

### 3.6 Metal Shader Optimizations
**File**: `crates/gpui/src/platform/mac/shaders.metal`
**Problem**: Several shader optimizations available:
1. SDF calculations for rounded rectangles are done per-pixel even for solid-color quads without rounded corners
2. Shadow blur uses Gaussian approximation per-pixel that could be pre-computed as a lookup texture
3. `half` precision could be used for colors and intermediate calculations where f32 is overkill
**Fix**:
1. Add fast-path for quads without rounded corners (skip SDF entirely)
2. Pre-compute Gaussian blur kernel as a 1D texture for shadow rendering
3. Use `half4` for color calculations in fragment shaders
**Impact**: MEDIUM - Improves GPU throughput, especially on lower-end Apple Silicon (M1, A-series).

---

## Priority Implementation Order

| Phase | Items | Combined Impact |
|-------|-------|----------------|
| **Phase 1** (1-2 days) | 1.1, 1.3, 1.5 | ~15-20% less allocation per frame |
| **Phase 2** (2-3 days) | 2.1, 2.3 | ~30-50% less render work for static UIs |
| **Phase 3** (3-5 days) | 2.4, 2.5, 1.2, 1.4 | ~20% GPU throughput improvement |
| **Phase 4** (1-2 weeks) | 3.1, 3.2 | ~5-10x improvement for idle/static UIs |
| **Phase 5** (1-2 weeks) | 3.3, 3.4, 3.5, 3.6 | ~30-50% improvement for complex/large UIs |

## Measurement Strategy

Before implementing, add profiling instrumentation:
1. `#[profiling::function]` annotations on hot paths (already present on `present()`)
2. Frame time breakdown: layout_ms, paint_ms, gpu_ms, total_ms
3. Per-frame counters: primitive_count, element_count, relayout_count, text_shape_count
4. Memory high-water mark tracking per frame

Recommended profiling tools:
- **Instruments.app** (Metal System Trace) for GPU-side bottlenecks
- **cargo-flamegraph** for CPU hotspots
- **DHAT** (via valgrind) or **dhat-rs** for allocation profiling
- **tracy** (via `profiling` crate) for frame-by-frame breakdown

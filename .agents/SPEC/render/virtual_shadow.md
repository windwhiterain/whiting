# Virtual Shadow

a `Texture Piramd` that each `Instance` write depth into the `Layer` that fit its `Required Resolution`.

## Sample 

from `Layer` with small size to large, once shadowed or masked by `Sparsity`, break.

## Terms

### `Required Resolution`

texels per world length.

### `Sparse Texture`

a virtual texture spliced into multiple tiles, each tile can be a real texture or none.

### `Texture Piramid`

muliple `Layer`s of `Sparse Texture` with size from small to large.

each `Layer` has a `Sparsity` bitmask for larger layer.

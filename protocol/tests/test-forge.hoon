::  protocol/tests/test-forge.hoon: Forge component tests
::
::  Tests vesl-merkle primitives in forge-relevant scenarios.
::  No kernel boot — compile-time assertion tests only.
::  Compilation success = all assertions passed.
::
/+  *vesl-merkle
::
::  ============================================
::  TEST 1: Single-leaf verify roundtrip
::  ============================================
::
::  A single leaf's hash IS the root.  verify-chunk with an
::  empty proof should confirm the leaf matches.
::
=/  leaf-1=@  'forge-test-alpha'
=/  root-1=@  (hash-leaf leaf-1)
::
?>  (verify-chunk leaf-1 ~ root-1)
::
::  ============================================
::  TEST 2: Two-leaf Merkle tree
::  ============================================
::
::  Two leaves, hash each, pair-hash for root.
::  Verify each with its sibling as proof node.
::
=/  leaf-a=@  'left-leaf'
=/  leaf-b=@  'right-leaf'
=/  hash-a=@  (hash-leaf leaf-a)
=/  hash-b=@  (hash-leaf leaf-b)
=/  root-2=@  (hash-pair hash-a hash-b)
::
::  Left leaf (side=%.y → sibling is right, prepend hash-b)
::
?>  (verify-chunk leaf-a ~[[hash=hash-b side=%.y]] root-2)
::
::  Right leaf (side=%.n → sibling is left, append hash-a)
::
?>  (verify-chunk leaf-b ~[[hash=hash-a side=%.n]] root-2)
::
::  ============================================
::  TEST 3: Belt decomposition + fold
::  ============================================
::
::  Split an atom to belts, fold with Goldilocks modular sum.
::  Deterministic: same input always produces same digest.
::
=/  test-atom=@  'belt-test-data-0123456789'
=/  belts-1=(list @)  (split-to-belts test-atom)
=/  belts-2=(list @)  (split-to-belts test-atom)
::
::  Goldilocks prime: p = 2^64 - 2^32 + 1
::
=/  p=@  (add (sub (bex 64) (bex 32)) 1)
=/  digest-1=@
  %+  roll  belts-1
  |=  [a=@ b=@]
  (mod (add a b) p)
=/  digest-2=@
  %+  roll  belts-2
  |=  [a=@ b=@]
  (mod (add a b) p)
::
?>  =(digest-1 digest-2)
?>  (lth digest-1 p)
::
::  ============================================
::  TEST 4: Belt roundtrip
::  ============================================
::
::  split-to-belts then belts-to-atom recovers the original.
::
=/  rt-atom=@  'roundtrip-payload'
=/  rt-belts=(list @)  (split-to-belts rt-atom)
=/  recovered=@  (belts-to-atom rt-belts)
::
?>  =(rt-atom recovered)
::
::  ============================================
::  TEST 5: Wrong root rejection
::  ============================================
::
::  verify-chunk with a flipped root must return %.n.
::
=/  good-root=@  (hash-leaf 'correct-data')
=/  bad-root=@  (hash-leaf 'wrong-data')
::
?>  =(%.n (verify-chunk 'correct-data' ~ bad-root))
::
::  ============================================
::  TEST 6: Empty leaf (zero atom)
::  ============================================
::
::  Zero is a valid atom.  split-to-belts 0 should produce
::  ~[0] and hash-leaf 0 should be verifiable.
::
=/  zero-belts=(list @)  (split-to-belts 0)
?>  ?=(^ zero-belts)
?>  =(0 i.zero-belts)
::
=/  zero-root=@  (hash-leaf 0)
?>  (verify-chunk 0 ~ zero-root)
::
%pass
